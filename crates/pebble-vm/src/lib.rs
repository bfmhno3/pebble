#![forbid(unsafe_code)]
//! A stack-based virtual machine for Pebble bytecode.
//!
//! The VM is the second backend. Where the tree-walker in `pebble-interp` walks
//! the AST with `Rc<RefCell<..>>` environments (and therefore leaks recursive
//! closures through reference cycles), the VM allocates its runtime objects on
//! the [`pebble_gc::Heap`], so cycles are collectable.
//!
//! The value stack holds [`VmValue`]s. Scalars are inline; everything with a
//! dynamic shape (strings, lists, maps, environments, closures) is a
//! [`GcRef`] index into the heap. Before each instruction the VM may collect;
//! the roots are the value stack, every live frame's environment, and the
//! globals.

use std::rc::Rc;

use pebble_bytecode::{Chunk, Compiled, Constant, FunctionProto, Opcode};
use pebble_gc::{GcObject, GcRef, Heap};
use pebble_span::Span;
use pebble_value::{Output, RuntimeError, Stdout};

/// A VM value.
///
/// `Copy`, so pushing one around never clones a heap object; the heap object
/// lives behind the [`GcRef`].
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum VmValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    /// A reference to a heap object.
    Ref(GcRef),
}

impl VmValue {
    fn is_truthy(self) -> bool {
        !matches!(self, VmValue::Nil | VmValue::Bool(false))
    }

    fn as_ref(self) -> Option<GcRef> {
        match self {
            VmValue::Ref(reference) => Some(reference),
            _ => None,
        }
    }
}

/// A heap-allocated VM object.
pub enum VmObject {
    /// An immutable string.
    Str(String),
    /// A mutable list.
    List(Vec<VmValue>),
    /// A map, stored as insertion-ordered key/value pairs and searched linearly.
    Map(Vec<(VmValue, VmValue)>),
    /// A function value: a prototype plus the environment it closed over.
    Closure {
        proto: Rc<FunctionProto>,
        env: GcRef,
    },
    /// A frame of bindings with a link to the enclosing frame.
    Env {
        parent: Option<GcRef>,
        slots: Vec<(String, VmValue)>,
    },
    /// A builtin function, identified by name.
    Native(&'static str),
}

impl GcObject for VmObject {
    fn trace(&self, mark: &mut dyn FnMut(GcRef)) {
        match self {
            VmObject::Str(_) | VmObject::Native(_) => {}
            VmObject::List(items) => {
                for item in items {
                    if let VmValue::Ref(reference) = item {
                        mark(*reference);
                    }
                }
            }
            VmObject::Map(entries) => {
                for (key, value) in entries {
                    if let VmValue::Ref(reference) = key {
                        mark(*reference);
                    }
                    if let VmValue::Ref(reference) = value {
                        mark(*reference);
                    }
                }
            }
            VmObject::Closure { env, .. } => mark(*env),
            VmObject::Env { parent, slots } => {
                if let Some(parent) = parent {
                    mark(*parent);
                }
                for (_, value) in slots {
                    if let VmValue::Ref(reference) = value {
                        mark(*reference);
                    }
                }
            }
        }
    }
}

/// One activation record.
struct Frame {
    proto: Rc<FunctionProto>,
    env: GcRef,
    ip: usize,
}

/// The virtual machine.
pub struct Vm {
    heap: Heap<VmObject>,
    stack: Vec<VmValue>,
    frames: Vec<Frame>,
    globals: GcRef,
    out: Box<dyn Output>,
    max_frames: usize,
    gc_threshold: usize,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    /// Creates a VM with default limits, printing to standard output.
    #[must_use]
    pub fn new() -> Self {
        Self::with_output(Box::new(Stdout))
    }

    /// Creates a VM with a custom output sink.
    #[must_use]
    pub fn with_output(out: Box<dyn Output>) -> Self {
        let mut heap = Heap::new();
        let globals = heap.alloc(VmObject::Env {
            parent: None,
            slots: Vec::new(),
        });
        for name in pebble_value::builtin_names() {
            let native = heap.alloc(VmObject::Native(name));
            if let VmObject::Env { slots, .. } = heap.get_mut(globals) {
                slots.push((name.to_string(), VmValue::Ref(native)));
            }
        }
        Self {
            heap,
            stack: Vec::new(),
            frames: Vec::new(),
            globals,
            out,
            max_frames: 256,
            gc_threshold: 1024,
        }
    }

    /// Sets the recursion limit.
    pub fn set_max_frames(&mut self, frames: usize) {
        self.max_frames = frames;
    }

    /// Sets how many allocations may accumulate before a collection.
    pub fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold;
    }

    /// Number of live heap objects.
    #[must_use]
    pub fn heap_live(&self) -> usize {
        self.heap.live_count()
    }

    /// Estimated heap bytes.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        self.heap.bytes_allocated()
    }

    /// Runs a compiled program and returns its value.
    ///
    /// # Errors
    /// Returns a [`RuntimeError`] for any type error, undefined name, arity
    /// mismatch, or recursion-limit breach.
    pub fn run(&mut self, compiled: &Compiled) -> Result<VmValue, RuntimeError> {
        self.stack.clear();
        self.frames.clear();
        self.frames.push(Frame {
            proto: Rc::clone(&compiled.main),
            env: self.globals,
            ip: 0,
        });

        loop {
            if self.frames.is_empty() {
                return Ok(self.stack.pop().unwrap_or(VmValue::Nil));
            }
            if self.heap.allocations_since_gc() > self.gc_threshold {
                self.collect_garbage();
            }

            let frame_index = self.frames.len() - 1;
            let proto = Rc::clone(&self.frames[frame_index].proto);
            let ip = self.frames[frame_index].ip;

            if ip >= proto.chunk.len() {
                // Falling off the end of a chunk returns its top-of-stack.
                self.frames.pop();
                let result = self.stack.pop().unwrap_or(VmValue::Nil);
                if self.frames.is_empty() {
                    return Ok(result);
                }
                self.stack.push(result);
                continue;
            }

            let instruction = proto.chunk.code()[ip];
            self.frames[frame_index].ip = ip + 1;
            let span = proto.chunk.span_at(ip);

            match instruction {
                Opcode::Const(index) => self.push_constant(&proto.chunk, index, span)?,
                Opcode::Nil => self.stack.push(VmValue::Nil),
                Opcode::True => self.stack.push(VmValue::Bool(true)),
                Opcode::False => self.stack.push(VmValue::Bool(false)),
                Opcode::Pop => {
                    self.stack.pop();
                }
                Opcode::DefineName(index) => {
                    let name = self.constant_name(&proto.chunk, index);
                    let value = self.stack.pop().unwrap_or(VmValue::Nil);
                    self.define(name, value);
                }
                Opcode::GetName(index) => {
                    let name = self.constant_name(&proto.chunk, index);
                    let value = self.lookup(&name).ok_or_else(|| {
                        RuntimeError::new(span, format!("undefined variable `{name}`"))
                    })?;
                    self.stack.push(value);
                }
                Opcode::SetName(index) => {
                    let name = self.constant_name(&proto.chunk, index);
                    let value = *self.stack.last().unwrap_or(&VmValue::Nil);
                    if !self.assign(&name, value) {
                        return Err(RuntimeError::new(
                            span,
                            format!("cannot assign to undefined variable `{name}`"),
                        ));
                    }
                }
                Opcode::MakeClosure(index) => {
                    let Constant::Function(proto_of_closure) =
                        &proto.chunk.constants()[index as usize]
                    else {
                        return Err(RuntimeError::new(span, "malformed closure constant"));
                    };
                    let env = self.frames[frame_index].env;
                    let reference = self.heap.alloc(VmObject::Closure {
                        proto: Rc::clone(proto_of_closure),
                        env,
                    });
                    self.stack.push(VmValue::Ref(reference));
                }
                Opcode::Add => self.binary_add(span)?,
                Opcode::Sub => self.binary_numeric(span, "-", i64::checked_sub, |a, b| a - b)?,
                Opcode::Mul => self.binary_numeric(span, "*", i64::checked_mul, |a, b| a * b)?,
                Opcode::Div => self.binary_div(span)?,
                Opcode::Rem => self.binary_rem(span)?,
                Opcode::Neg => {
                    let value = self.pop(span)?;
                    let result = match value {
                        VmValue::Int(inner) => inner
                            .checked_neg()
                            .map(VmValue::Int)
                            .ok_or_else(|| self.overflow(span))?,
                        VmValue::Float(inner) => VmValue::Float(-inner),
                        other => {
                            return Err(RuntimeError::new(
                                span,
                                format!("cannot negate {}", self.type_name(other)),
                            ));
                        }
                    };
                    self.stack.push(result);
                }
                Opcode::Not => {
                    let value = self.pop(span)?;
                    self.stack.push(VmValue::Bool(!value.is_truthy()));
                }
                Opcode::Equal => {
                    let rhs = self.pop(span)?;
                    let lhs = self.pop(span)?;
                    let equal = self.value_equal(lhs, rhs);
                    self.stack.push(VmValue::Bool(equal));
                }
                Opcode::NotEqual => {
                    let rhs = self.pop(span)?;
                    let lhs = self.pop(span)?;
                    let equal = self.value_equal(lhs, rhs);
                    self.stack.push(VmValue::Bool(!equal));
                }
                Opcode::Less | Opcode::LessEqual | Opcode::Greater | Opcode::GreaterEqual => {
                    let ordering = self.binary_compare(span)?;
                    use std::cmp::Ordering;
                    let result = match instruction {
                        Opcode::Less => ordering == Ordering::Less,
                        Opcode::LessEqual => ordering != Ordering::Greater,
                        Opcode::Greater => ordering == Ordering::Greater,
                        Opcode::GreaterEqual => ordering != Ordering::Less,
                        _ => unreachable!(),
                    };
                    self.stack.push(VmValue::Bool(result));
                }
                Opcode::Jump(target) => self.frames[frame_index].ip = target as usize,
                Opcode::JumpIfFalse(target) => {
                    let value = self.pop(span)?;
                    if !value.is_truthy() {
                        self.frames[frame_index].ip = target as usize;
                    }
                }
                Opcode::JumpIfFalseKeep(target) => {
                    let value = *self.stack.last().unwrap_or(&VmValue::Nil);
                    if value.is_truthy() {
                        self.stack.pop();
                    } else {
                        self.frames[frame_index].ip = target as usize;
                    }
                }
                Opcode::JumpIfTrueKeep(target) => {
                    let value = *self.stack.last().unwrap_or(&VmValue::Nil);
                    if value.is_truthy() {
                        self.frames[frame_index].ip = target as usize;
                    } else {
                        self.stack.pop();
                    }
                }
                Opcode::Call(count) => self.call(count, span)?,
                Opcode::Return => {
                    let value = self.stack.pop().unwrap_or(VmValue::Nil);
                    self.frames.pop();
                    if self.frames.is_empty() {
                        return Ok(value);
                    }
                    self.stack.push(value);
                }
                Opcode::BuildList(count) => {
                    let items = self.stack.split_off(self.stack.len() - count as usize);
                    let reference = self.heap.alloc(VmObject::List(items));
                    self.stack.push(VmValue::Ref(reference));
                }
                Opcode::BuildMap(count) => {
                    let flat = self
                        .stack
                        .split_off(self.stack.len() - (count as usize * 2));
                    let mut entries = Vec::with_capacity(count as usize);
                    let mut iter = flat.into_iter();
                    while let (Some(key), Some(value)) = (iter.next(), iter.next()) {
                        entries.push((key, value));
                    }
                    let reference = self.heap.alloc(VmObject::Map(entries));
                    self.stack.push(VmValue::Ref(reference));
                }
                Opcode::Index => {
                    let index = self.pop(span)?;
                    let container = self.pop(span)?;
                    let result = self.index_value(container, index, span)?;
                    self.stack.push(result);
                }
                Opcode::SetIndex => {
                    let value = self.pop(span)?;
                    let index = self.pop(span)?;
                    let container = self.pop(span)?;
                    self.set_index_value(container, index, value, span)?;
                    self.stack.push(value);
                }
            }
        }
    }

    // --- helpers: stack, constants ---------------------------------------

    fn pop(&mut self, span: Span) -> Result<VmValue, RuntimeError> {
        self.stack
            .pop()
            .ok_or_else(|| RuntimeError::new(span, "internal error: value stack underflow"))
    }

    fn push_constant(&mut self, chunk: &Chunk, index: u32, span: Span) -> Result<(), RuntimeError> {
        match &chunk.constants()[index as usize] {
            Constant::Nil => self.stack.push(VmValue::Nil),
            Constant::Bool(value) => self.stack.push(VmValue::Bool(*value)),
            Constant::Int(value) => self.stack.push(VmValue::Int(*value)),
            Constant::Float(value) => self.stack.push(VmValue::Float(*value)),
            Constant::Str(value) => {
                let reference = self.heap.alloc(VmObject::Str(value.clone()));
                self.stack.push(VmValue::Ref(reference));
            }
            Constant::Function(_) => {
                return Err(RuntimeError::new(span, "function constant used as a value"));
            }
        }
        Ok(())
    }

    fn constant_name(&self, chunk: &Chunk, index: u32) -> String {
        match &chunk.constants()[index as usize] {
            Constant::Str(name) => name.clone(),
            _ => format!("<constant {index}>"),
        }
    }

    // --- helpers: environments -------------------------------------------

    fn current_env(&self) -> GcRef {
        self.frames.last().map_or(self.globals, |frame| frame.env)
    }

    fn define(&mut self, name: String, value: VmValue) {
        let env = self.current_env();
        if let VmObject::Env { slots, .. } = self.heap.get_mut(env) {
            if let Some(slot) = slots.iter_mut().find(|(existing, _)| *existing == name) {
                slot.1 = value;
            } else {
                slots.push((name, value));
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<VmValue> {
        let mut current = Some(self.current_env());
        while let Some(env) = current {
            let VmObject::Env { parent, slots } = self.heap.get(env) else {
                return None;
            };
            if let Some((_, value)) = slots.iter().find(|(existing, _)| existing == name) {
                return Some(*value);
            }
            current = *parent;
        }
        None
    }

    fn assign(&mut self, name: &str, value: VmValue) -> bool {
        let mut current = Some(self.current_env());
        while let Some(env) = current {
            if let VmObject::Env { parent, slots } = self.heap.get_mut(env) {
                if let Some(slot) = slots.iter_mut().find(|(existing, _)| existing == name) {
                    slot.1 = value;
                    return true;
                }
                current = *parent;
            } else {
                return false;
            }
        }
        false
    }

    // --- helpers: calls ---------------------------------------------------

    fn call(&mut self, count: u32, span: Span) -> Result<(), RuntimeError> {
        let count = count as usize;
        if self.stack.len() < count + 1 {
            return Err(RuntimeError::new(
                span,
                "internal error: call stack underflow",
            ));
        }
        let args = self.stack.split_off(self.stack.len() - count);
        let callee = self.pop(span)?;

        enum Target {
            Native(&'static str),
            Closure(Rc<FunctionProto>, GcRef),
        }

        let target = match callee.as_ref().map(|reference| self.heap.get(reference)) {
            Some(VmObject::Native(name)) => Target::Native(name),
            Some(VmObject::Closure { proto, env }) => Target::Closure(Rc::clone(proto), *env),
            _ => {
                return Err(RuntimeError::new(
                    span,
                    format!("{} is not callable", self.type_name(callee)),
                ));
            }
        };

        match target {
            Target::Native(name) => {
                let result = self.call_builtin(name, &args, span)?;
                self.stack.push(result);
            }
            Target::Closure(proto, env) => {
                if args.len() != proto.params.len() {
                    return Err(RuntimeError::new(
                        span,
                        format!(
                            "{} expects {} argument(s), got {}",
                            proto.display_name(),
                            proto.params.len(),
                            args.len()
                        ),
                    ));
                }
                if self.frames.len() >= self.max_frames {
                    return Err(RuntimeError::new(
                        span,
                        format!("call stack overflow (limit {})", self.max_frames),
                    ));
                }
                let slots = proto.params.iter().cloned().zip(args).collect();
                let new_env = self.heap.alloc(VmObject::Env {
                    parent: Some(env),
                    slots,
                });
                self.frames.push(Frame {
                    proto,
                    env: new_env,
                    ip: 0,
                });
            }
        }
        Ok(())
    }

    fn call_builtin(
        &mut self,
        name: &str,
        args: &[VmValue],
        span: Span,
    ) -> Result<VmValue, RuntimeError> {
        match name {
            "print" => {
                let text = args
                    .iter()
                    .map(|value| self.display(*value))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.out.write_line(&text);
                Ok(VmValue::Nil)
            }
            "len" => {
                self.expect_args(args, 1, span)?;
                let len = match args[0].as_ref().map(|reference| self.heap.get(reference)) {
                    Some(VmObject::Str(text)) => text.chars().count(),
                    Some(VmObject::List(items)) => items.len(),
                    Some(VmObject::Map(entries)) => entries.len(),
                    _ => {
                        return Err(RuntimeError::new(
                            span,
                            format!("`len` does not accept {}", self.type_name(args[0])),
                        ));
                    }
                };
                Ok(VmValue::Int(len as i64))
            }
            "type" => {
                self.expect_args(args, 1, span)?;
                Ok(self.allocate_string(self.type_name(args[0]).to_string()))
            }
            "str" => {
                self.expect_args(args, 1, span)?;
                let text = self.display(args[0]);
                Ok(self.allocate_string(text))
            }
            "int" => {
                self.expect_args(args, 1, span)?;
                Ok(match args[0] {
                    VmValue::Int(value) => VmValue::Int(value),
                    VmValue::Float(value) => VmValue::Int(value as i64),
                    VmValue::Bool(value) => VmValue::Int(i64::from(u8::from(value))),
                    other => {
                        let text = self.display(other);
                        text.trim().parse::<i64>().map(VmValue::Int).map_err(|_| {
                            RuntimeError::new(span, format!("cannot parse {text:?} as an int"))
                        })?
                    }
                })
            }
            "float" => {
                self.expect_args(args, 1, span)?;
                Ok(match args[0] {
                    VmValue::Int(value) => VmValue::Float(value as f64),
                    VmValue::Float(value) => VmValue::Float(value),
                    other => {
                        let text = self.display(other);
                        text.trim()
                            .parse::<f64>()
                            .map(VmValue::Float)
                            .map_err(|_| {
                                RuntimeError::new(span, format!("cannot parse {text:?} as a float"))
                            })?
                    }
                })
            }
            "range" => {
                let (start, end) = match args.len() {
                    1 => (0, self.expect_int(args[0], span)?),
                    2 => (
                        self.expect_int(args[0], span)?,
                        self.expect_int(args[1], span)?,
                    ),
                    _ => {
                        return Err(RuntimeError::new(span, "`range` expects 1 or 2 arguments"));
                    }
                };
                let items = (start..end).map(VmValue::Int).collect();
                let reference = self.heap.alloc(VmObject::List(items));
                Ok(VmValue::Ref(reference))
            }
            "push" => {
                self.expect_args(args, 2, span)?;
                let Some(reference) = args[0].as_ref() else {
                    return Err(RuntimeError::new(span, "`push` expects a list"));
                };
                let item = args[1];
                match self.heap.get_mut(reference) {
                    VmObject::List(items) => items.push(item),
                    _ => return Err(RuntimeError::new(span, "`push` expects a list")),
                }
                Ok(VmValue::Nil)
            }
            "keys" => {
                self.expect_args(args, 1, span)?;
                let Some(reference) = args[0].as_ref() else {
                    return Err(RuntimeError::new(span, "`keys` expects a map"));
                };
                let VmObject::Map(entries) = self.heap.get(reference) else {
                    return Err(RuntimeError::new(span, "`keys` expects a map"));
                };
                let keys = entries.iter().map(|(key, _)| *key).collect();
                let result = self.heap.alloc(VmObject::List(keys));
                Ok(VmValue::Ref(result))
            }
            "clock" => {
                let seconds = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_secs_f64())
                    .unwrap_or(0.0);
                Ok(VmValue::Float(seconds))
            }
            "assert" => {
                if args.first().is_some_and(|value| value.is_truthy()) {
                    return Ok(VmValue::Nil);
                }
                let message = args.get(1).map_or_else(
                    || "assertion failed".to_string(),
                    |value| self.display(*value),
                );
                self.out.write_line(&message);
                Err(RuntimeError::new(span, message))
            }
            other => Err(RuntimeError::new(
                span,
                format!("unknown builtin `{other}`"),
            )),
        }
    }

    fn expect_args(&self, args: &[VmValue], count: usize, span: Span) -> Result<(), RuntimeError> {
        if args.len() == count {
            Ok(())
        } else {
            Err(RuntimeError::new(
                span,
                format!("expected {count} argument(s), got {}", args.len()),
            ))
        }
    }

    fn expect_int(&self, value: VmValue, span: Span) -> Result<i64, RuntimeError> {
        match value {
            VmValue::Int(inner) => Ok(inner),
            other => Err(RuntimeError::new(
                span,
                format!("expected an int, got {}", self.type_name(other)),
            )),
        }
    }

    fn allocate_string(&mut self, text: String) -> VmValue {
        let reference = self.heap.alloc(VmObject::Str(text));
        VmValue::Ref(reference)
    }

    // --- helpers: operators ----------------------------------------------

    fn overflow(&self, span: Span) -> RuntimeError {
        RuntimeError::new(span, "integer overflow")
    }

    fn binary_add(&mut self, span: Span) -> Result<(), RuntimeError> {
        let rhs = self.pop(span)?;
        let lhs = self.pop(span)?;
        let result = match (lhs, rhs) {
            (VmValue::Int(a), VmValue::Int(b)) => a
                .checked_add(b)
                .map(VmValue::Int)
                .ok_or_else(|| self.overflow(span))?,
            (VmValue::Int(a), VmValue::Float(b)) => VmValue::Float(a as f64 + b),
            (VmValue::Float(a), VmValue::Int(b)) => VmValue::Float(a + b as f64),
            (VmValue::Float(a), VmValue::Float(b)) => VmValue::Float(a + b),
            (a, b) if self.is_str(a) && self.is_str(b) => {
                let mut text = self.as_string(a);
                text.push_str(&self.as_string(b));
                self.allocate_string(text)
            }
            (a, b) if self.is_list(a) && self.is_list(b) => {
                let mut items = self.as_list(a);
                items.extend(self.as_list(b));
                let reference = self.heap.alloc(VmObject::List(items));
                VmValue::Ref(reference)
            }
            (a, b) => {
                return Err(RuntimeError::new(
                    span,
                    format!(
                        "cannot apply `+` to {} and {}",
                        self.type_name(a),
                        self.type_name(b)
                    ),
                ));
            }
        };
        self.stack.push(result);
        Ok(())
    }

    fn binary_numeric(
        &mut self,
        span: Span,
        op: &str,
        int_op: fn(i64, i64) -> Option<i64>,
        float_op: fn(f64, f64) -> f64,
    ) -> Result<(), RuntimeError> {
        let rhs = self.pop(span)?;
        let lhs = self.pop(span)?;
        let result = match (lhs, rhs) {
            (VmValue::Int(a), VmValue::Int(b)) => int_op(a, b)
                .map(VmValue::Int)
                .ok_or_else(|| self.overflow(span))?,
            (a, b) if self.is_number(a) && self.is_number(b) => {
                VmValue::Float(float_op(self.as_f64(a), self.as_f64(b)))
            }
            (a, b) => {
                return Err(RuntimeError::new(
                    span,
                    format!(
                        "cannot apply `{op}` to {} and {}",
                        self.type_name(a),
                        self.type_name(b)
                    ),
                ));
            }
        };
        self.stack.push(result);
        Ok(())
    }

    fn binary_div(&mut self, span: Span) -> Result<(), RuntimeError> {
        let rhs = self.pop(span)?;
        let lhs = self.pop(span)?;
        match (lhs, rhs) {
            (VmValue::Int(a), VmValue::Int(b)) => {
                if b == 0 {
                    return Err(RuntimeError::new(span, "division by zero"));
                }
                let result = a.checked_div(b).ok_or_else(|| self.overflow(span))?;
                self.stack.push(VmValue::Int(result));
            }
            (a, b) if self.is_number(a) && self.is_number(b) => {
                self.stack
                    .push(VmValue::Float(self.as_f64(a) / self.as_f64(b)));
            }
            (a, b) => {
                return Err(RuntimeError::new(
                    span,
                    format!(
                        "cannot apply `/` to {} and {}",
                        self.type_name(a),
                        self.type_name(b)
                    ),
                ));
            }
        }
        Ok(())
    }

    fn binary_rem(&mut self, span: Span) -> Result<(), RuntimeError> {
        let rhs = self.pop(span)?;
        let lhs = self.pop(span)?;
        if let (VmValue::Int(a), VmValue::Int(b)) = (lhs, rhs) {
            if b == 0 {
                return Err(RuntimeError::new(span, "remainder by zero"));
            }
            let result = a.checked_rem(b).ok_or_else(|| self.overflow(span))?;
            self.stack.push(VmValue::Int(result));
            return Ok(());
        }
        if self.is_number(lhs) && self.is_number(rhs) {
            let a = self.as_f64(lhs);
            let b = self.as_f64(rhs);
            self.stack.push(VmValue::Float(a % b));
            return Ok(());
        }
        Err(RuntimeError::new(
            span,
            format!(
                "cannot apply `%` to {} and {}",
                self.type_name(lhs),
                self.type_name(rhs)
            ),
        ))
    }

    fn binary_compare(&mut self, span: Span) -> Result<std::cmp::Ordering, RuntimeError> {
        let rhs = self.pop(span)?;
        let lhs = self.pop(span)?;
        match (lhs, rhs) {
            (VmValue::Int(a), VmValue::Int(b)) => Ok(a.cmp(&b)),
            (a, b) if self.is_str(a) && self.is_str(b) => {
                Ok(self.as_string(a).cmp(&self.as_string(b)))
            }
            (a, b) if self.is_number(a) && self.is_number(b) => {
                let a = self.as_f64(a);
                let b = self.as_f64(b);
                a.partial_cmp(&b)
                    .ok_or_else(|| RuntimeError::new(span, "cannot compare with NaN"))
            }
            (a, b) => Err(RuntimeError::new(
                span,
                format!(
                    "cannot compare {} and {}",
                    self.type_name(a),
                    self.type_name(b)
                ),
            )),
        }
    }

    fn is_number(&self, value: VmValue) -> bool {
        matches!(value, VmValue::Int(_) | VmValue::Float(_))
    }

    fn as_f64(&self, value: VmValue) -> f64 {
        match value {
            VmValue::Int(inner) => inner as f64,
            VmValue::Float(inner) => inner,
            _ => f64::NAN,
        }
    }

    fn is_str(&self, value: VmValue) -> bool {
        matches!(
            value.as_ref().map(|reference| self.heap.get(reference)),
            Some(VmObject::Str(_))
        )
    }

    fn is_list(&self, value: VmValue) -> bool {
        matches!(
            value.as_ref().map(|reference| self.heap.get(reference)),
            Some(VmObject::List(_))
        )
    }

    fn as_string(&self, value: VmValue) -> String {
        match value.as_ref().map(|reference| self.heap.get(reference)) {
            Some(VmObject::Str(text)) => text.clone(),
            _ => String::new(),
        }
    }

    fn as_list(&self, value: VmValue) -> Vec<VmValue> {
        match value.as_ref().map(|reference| self.heap.get(reference)) {
            Some(VmObject::List(items)) => items.clone(),
            _ => Vec::new(),
        }
    }

    fn value_equal(&self, lhs: VmValue, rhs: VmValue) -> bool {
        match (lhs, rhs) {
            (VmValue::Nil, VmValue::Nil) => true,
            (VmValue::Bool(a), VmValue::Bool(b)) => a == b,
            (VmValue::Int(a), VmValue::Int(b)) => a == b,
            (VmValue::Int(a), VmValue::Float(b)) | (VmValue::Float(b), VmValue::Int(a)) => {
                a as f64 == b
            }
            (VmValue::Float(a), VmValue::Float(b)) => a == b,
            (VmValue::Ref(a), VmValue::Ref(b)) => {
                match (self.heap.get(a), self.heap.get(b)) {
                    (VmObject::Str(x), VmObject::Str(y)) => x == y,
                    // Lists, maps, closures and envs compare by identity.
                    _ => a == b,
                }
            }
            _ => false,
        }
    }

    fn type_name(&self, value: VmValue) -> &'static str {
        match value {
            VmValue::Nil => "nil",
            VmValue::Bool(_) => "bool",
            VmValue::Int(_) => "int",
            VmValue::Float(_) => "float",
            VmValue::Ref(reference) => match self.heap.get(reference) {
                VmObject::Str(_) => "string",
                VmObject::List(_) => "list",
                VmObject::Map(_) => "map",
                VmObject::Closure { .. } | VmObject::Native(_) => "function",
                VmObject::Env { .. } => "env",
            },
        }
    }

    fn index_value(
        &mut self,
        container: VmValue,
        index: VmValue,
        span: Span,
    ) -> Result<VmValue, RuntimeError> {
        enum Kind {
            List(Vec<VmValue>),
            Str(Vec<char>),
            Map(Vec<(VmValue, VmValue)>),
            Other,
        }

        let Some(reference) = container.as_ref() else {
            return Err(RuntimeError::new(
                span,
                format!("cannot index into {}", self.type_name(container)),
            ));
        };
        // Snapshot the contents so the heap borrow ends before we allocate.
        let kind = match self.heap.get(reference) {
            VmObject::List(items) => Kind::List(items.clone()),
            VmObject::Str(text) => Kind::Str(text.chars().collect()),
            VmObject::Map(entries) => Kind::Map(entries.clone()),
            _ => Kind::Other,
        };

        match kind {
            Kind::List(items) => {
                let position = self.resolve_index(items.len(), index, span)?;
                Ok(items[position])
            }
            Kind::Str(chars) => {
                let position = self.resolve_index(chars.len(), index, span)?;
                Ok(self.allocate_string(chars[position].to_string()))
            }
            Kind::Map(entries) => entries
                .iter()
                .find(|(existing, _)| self.value_equal(*existing, index))
                .map(|(_, value)| *value)
                .ok_or_else(|| {
                    RuntimeError::new(span, format!("key {} not in map", self.display(index)))
                }),
            Kind::Other => Err(RuntimeError::new(
                span,
                format!("cannot index into {}", self.type_name(container)),
            )),
        }
    }

    fn set_index_value(
        &mut self,
        container: VmValue,
        index: VmValue,
        value: VmValue,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let Some(reference) = container.as_ref() else {
            return Err(RuntimeError::new(
                span,
                format!("cannot assign into {}", self.type_name(container)),
            ));
        };
        let length = match self.heap.get(reference) {
            VmObject::List(items) => Some(items.len()),
            VmObject::Map(_) => None,
            _ => {
                return Err(RuntimeError::new(
                    span,
                    format!("cannot assign into {}", self.type_name(container)),
                ));
            }
        };
        match length {
            Some(len) => {
                let position = self.resolve_index(len, index, span)?;
                if let VmObject::List(items) = self.heap.get_mut(reference) {
                    items[position] = value;
                }
                Ok(())
            }
            None => {
                // Find the matching slot first, so the heap borrow is released
                // before we mutate (and before asking `self` anything again).
                let existing = match self.heap.get(reference) {
                    VmObject::Map(entries) => entries
                        .iter()
                        .position(|(existing, _)| self.value_equal(*existing, index)),
                    _ => None,
                };
                if let VmObject::Map(entries) = self.heap.get_mut(reference) {
                    match existing {
                        Some(position) => entries[position].1 = value,
                        None => entries.push((index, value)),
                    }
                }
                Ok(())
            }
        }
    }

    fn resolve_index(&self, len: usize, index: VmValue, span: Span) -> Result<usize, RuntimeError> {
        let VmValue::Int(raw) = index else {
            return Err(RuntimeError::new(
                span,
                format!("index must be an int, got {}", self.type_name(index)),
            ));
        };
        let len = len as i64;
        let position = if raw < 0 { len + raw } else { raw };
        if position < 0 || position >= len {
            return Err(RuntimeError::new(
                span,
                format!("index {raw} is out of range for length {len}"),
            ));
        }
        Ok(position as usize)
    }

    /// Renders a value, guarding against cycles.
    #[must_use]
    pub fn display(&self, value: VmValue) -> String {
        let mut out = String::new();
        self.write_display(&mut out, value, 0);
        out
    }

    fn write_display(&self, out: &mut String, value: VmValue, depth: usize) {
        const MAX_DEPTH: usize = 4;
        match value {
            VmValue::Nil => out.push_str("nil"),
            VmValue::Bool(inner) => out.push_str(if inner { "true" } else { "false" }),
            VmValue::Int(inner) => out.push_str(&inner.to_string()),
            VmValue::Float(inner) => {
                if inner.is_finite() && inner.fract() == 0.0 {
                    out.push_str(&format!("{inner:.1}"));
                } else {
                    out.push_str(&inner.to_string());
                }
            }
            VmValue::Ref(reference) => match self.heap.get(reference) {
                VmObject::Str(text) => out.push_str(text),
                VmObject::List(items) => {
                    if depth >= MAX_DEPTH {
                        out.push_str("[...]");
                        return;
                    }
                    out.push('[');
                    for (index, item) in items.iter().enumerate() {
                        if index > 0 {
                            out.push_str(", ");
                        }
                        self.write_display(out, *item, depth + 1);
                    }
                    out.push(']');
                }
                VmObject::Map(entries) => {
                    if depth >= MAX_DEPTH {
                        out.push_str("#{...}");
                        return;
                    }
                    out.push_str("#{");
                    for (index, (key, item)) in entries.iter().enumerate() {
                        if index > 0 {
                            out.push_str(", ");
                        }
                        self.write_display(out, *key, depth + 1);
                        out.push_str(": ");
                        self.write_display(out, *item, depth + 1);
                    }
                    out.push('}');
                }
                VmObject::Closure { proto, .. } => {
                    out.push_str(&format!("<fn {}>", proto.display_name()));
                }
                VmObject::Native(name) => out.push_str(&format!("<native fn {name}>")),
                VmObject::Env { .. } => out.push_str("<env>"),
            },
        }
    }

    // --- garbage collection ----------------------------------------------

    /// Marks every root and sweeps.
    ///
    /// Called between instructions, the only point where every live value is
    /// reachable from the stack, a frame, or the globals.
    pub fn collect_garbage(&mut self) {
        for value in &self.stack {
            if let VmValue::Ref(reference) = value {
                self.heap.mark(*reference);
            }
        }
        for frame in &self.frames {
            self.heap.mark(frame.env);
        }
        self.heap.mark(self.globals);
        self.heap.collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pebble_parser::parse;
    use pebble_value::Buffer;
    use std::cell::RefCell;

    struct Shared(Rc<RefCell<Buffer>>);

    impl Output for Shared {
        fn write_line(&mut self, line: &str) {
            self.0.borrow_mut().write_line(line);
        }
    }

    /// Compiles and runs, returning the VM (for `display`), its result, and the
    /// captured output.
    fn run_full(src: &str) -> Result<(Vm, VmValue, Vec<String>), String> {
        let shared = Rc::new(RefCell::new(Buffer::new()));
        let mut sources = pebble_span::SourceMap::new();
        let id = sources.add("<test>", src);
        let parsed = parse(id, src);
        if !parsed.is_ok() {
            return Err(parsed
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.render(&sources))
                .collect::<Vec<_>>()
                .join("\n"));
        }
        let compiled = pebble_bytecode::compile(&parsed.program);
        if !compiled.is_ok() {
            return Err(compiled
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.render(&sources))
                .collect::<Vec<_>>()
                .join("\n"));
        }
        let mut vm = Vm::with_output(Box::new(Shared(Rc::clone(&shared))));
        let value = vm
            .run(&compiled.compiled)
            .map_err(|error| error.diagnostic.render(&sources))?;
        let lines = shared.borrow().lines().to_vec();
        Ok((vm, value, lines))
    }

    fn run(src: &str) -> Result<VmValue, String> {
        run_full(src).map(|(_, value, _)| value)
    }

    fn run_display(src: &str) -> Result<String, String> {
        run_full(src).map(|(vm, value, _)| vm.display(value))
    }

    fn run_output(src: &str) -> Result<Vec<String>, String> {
        run_full(src).map(|(_, _, lines)| lines)
    }

    #[test]
    fn arithmetic_matches_the_tree_walker() {
        assert_eq!(run("1 + 2 * 3;").unwrap(), VmValue::Int(7));
        assert_eq!(run("(1 + 2) * 3;").unwrap(), VmValue::Int(9));
        assert_eq!(run("7 % 3;").unwrap(), VmValue::Int(1));
    }

    #[test]
    fn variables_and_assignment() {
        assert_eq!(run("let x = 1; x = x + 41; x;").unwrap(), VmValue::Int(42));
    }

    #[test]
    fn recursion_via_global_function() {
        let src = "
            fn fact(n) {
                if n < 2 { return 1; }
                return n * fact(n - 1);
            }
            fact(10);
        ";
        assert_eq!(run(src).unwrap(), VmValue::Int(3_628_800));
    }

    #[test]
    fn closures_capture_their_environment() {
        let src = "
            fn make_counter() {
                let n = 0;
                return fn() { n = n + 1; return n; };
            }
            let c = make_counter();
            c(); c(); c();
        ";
        assert_eq!(run(src).unwrap(), VmValue::Int(3));
    }

    #[test]
    fn lists_maps_and_indexing() {
        assert_eq!(
            run("let xs = [10, 20, 30]; xs[1];").unwrap(),
            VmValue::Int(20)
        );
        assert_eq!(
            run("let xs = [10, 20, 30]; xs[-1];").unwrap(),
            VmValue::Int(30)
        );
        assert_eq!(
            run("let m = #{ \"a\": 1 }; m[\"a\"];").unwrap(),
            VmValue::Int(1)
        );
        assert_eq!(
            run("let xs = [1]; xs[0] = 5; xs[0];").unwrap(),
            VmValue::Int(5)
        );
    }

    #[test]
    fn strings_concatenate_and_index() {
        assert_eq!(run_display(r#""foo" + "bar";"#).unwrap(), "foobar");
        assert_eq!(run_display(r#""abc"[2];"#).unwrap(), "c");
    }

    #[test]
    fn control_flow_matches_the_tree_walker() {
        let src = "
            let total = 0;
            for i in range(10) {
                if i == 3 { continue; }
                if i == 7 { break; }
                total = total + i;
            }
            total;
        ";
        assert_eq!(run(src).unwrap(), VmValue::Int(18));
    }

    #[test]
    fn while_loop_accumulates() {
        let src = "let i = 0; let s = 0; while i < 5 { s = s + i; i = i + 1; } s;";
        assert_eq!(run(src).unwrap(), VmValue::Int(10));
    }

    #[test]
    fn print_reaches_the_output_sink() {
        let lines = run_output("print(\"hello\", 1 + 1);").unwrap();
        assert_eq!(lines, vec!["hello 2".to_string()]);
    }

    #[test]
    fn division_by_zero_is_an_error() {
        assert!(run("1 / 0;").is_err());
    }

    #[test]
    fn arity_errors_are_reported() {
        let error = run("fn f(a) { return a; } f();").unwrap_err();
        assert!(error.contains("expects 1 argument"), "{error}");
    }

    #[test]
    fn undefined_variables_are_reported() {
        assert!(run("missing;").is_err());
    }

    #[test]
    fn recursion_limit_is_enforced() {
        // Run on a big stack: the VM recursion is heap-allocated but the Rust
        // frames are not.
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let error = run("fn loop() { return loop(); } loop();").unwrap_err();
                assert!(error.contains("call stack overflow"), "{error}");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn garbage_collection_reclaims_cycles() {
        let src = "
            fn make() {
                fn inner() { return inner; }
                return 1;
            }
            make();
        ";
        let mut sources = pebble_span::SourceMap::new();
        let id = sources.add("<test>", src);
        let parsed = parse(id, src);
        let compiled = pebble_bytecode::compile(&parsed.program);
        let mut vm = Vm::with_output(Box::new(Buffer::new()));
        vm.set_gc_threshold(usize::MAX);
        vm.run(&compiled.compiled).unwrap();
        let before = vm.heap_live();
        vm.collect_garbage();
        let after = vm.heap_live();
        assert_eq!(
            before - after,
            2,
            "make's call env and `inner` form an unreachable cycle"
        );
        vm.collect_garbage();
        assert_eq!(vm.heap_live(), after);
    }
}
