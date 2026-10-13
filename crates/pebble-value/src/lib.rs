#![forbid(unsafe_code)]
// Maps are keyed by `MapKey`, which wraps a `Value`. Clippy warns about
// interior-mutable key types, but our hashing is stable by construction:
// mutable values (lists, maps, closures) hash by pointer identity, and the
// scalar variants are immutable. See `Value::key_hash`.
#![allow(clippy::mutable_key_type)]
//! The Pebble runtime object model: values, environments, and builtin functions.
//!
//! This crate is where the interesting pointers live:
//!
//! * [`Rc`] for shared ownership — two variables can name the same list.
//! * [`RefCell`] for **interior mutability** — a list is mutated through a
//!   shared `Rc`, which the borrow checker alone would forbid.
//! * `Weak` is *not* used here on purpose: closures capture their defining
//!   environment with a strong `Rc`, which means a recursive `fn` stored in the
//!   environment it captures forms a reference cycle and leaks. That leak is
//!   exactly the problem the garbage-collected `pebble_vm` backend solves.
//!
//! Values are also where operator semantics live, because `+` has to mean
//! "add two ints" *and* "concatenate two strings" *and* "concatenate two lists".

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use pebble_ast::Stmt;
use pebble_diag::Diagnostic;
use pebble_span::Span;

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

/// A Pebble runtime value.
///
/// Heap-allocated variants share through [`Rc`]; the mutable ones wrap their
/// contents in [`RefCell`] so mutation is checked at run time rather than at
/// compile time.
#[derive(Clone)]
pub enum Value {
    /// `nil`
    Nil,
    /// `true` / `false`
    Bool(bool),
    /// A 64-bit signed integer.
    Int(i64),
    /// A 64-bit float.
    Float(f64),
    /// An immutable, shared string.
    Str(Rc<str>),
    /// A mutable, shared list.
    List(Rc<RefCell<Vec<Value>>>),
    /// A mutable, shared map.
    Map(Rc<RefCell<HashMap<MapKey, Value>>>),
    /// A user-defined function or closure.
    Function(Rc<Function>),
    /// A builtin implemented in Rust.
    Native(&'static NativeFunction),
}

impl Value {
    /// Builds a string value.
    #[must_use]
    pub fn string(text: impl Into<Rc<str>>) -> Self {
        Value::Str(text.into())
    }

    /// Builds a list value.
    #[must_use]
    pub fn list(items: Vec<Value>) -> Self {
        Value::List(Rc::new(RefCell::new(items)))
    }

    /// Builds an empty map value.
    #[must_use]
    pub fn map() -> Self {
        Value::Map(Rc::new(RefCell::new(HashMap::new())))
    }

    /// The value's type name, as `type()` reports it.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Function(_) | Value::Native(_) => "function",
        }
    }

    /// Truthiness: only `nil` and `false` are falsy.
    #[must_use]
    pub const fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }

    /// The language `==` operator.
    ///
    /// Numbers compare across `int`/`float`; strings compare by contents;
    /// everything else (lists, maps, functions) compares by identity, so two
    /// equal-looking lists are *not* `==` unless they are the same list.
    #[must_use]
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
                (*a as f64) == *b
            }
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(a, b),
            (Value::Map(a), Value::Map(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Native(a), Value::Native(b)) => core::ptr::eq(*a, *b),
            _ => false,
        }
    }

    /// Equality used for map keys.
    ///
    /// Differs from [`Value::equals`] in one place: floats compare by bits, so
    /// `NaN` is a usable, reflexive key. That is what `Eq` requires and what a
    /// `HashMap` needs.
    fn key_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            _ => self.equals(other),
        }
    }

    /// Hashing consistent with [`Value::key_eq`].
    fn key_hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Nil => 0u8.hash(state),
            Value::Bool(value) => {
                1u8.hash(state);
                value.hash(state);
            }
            Value::Int(value) => {
                2u8.hash(state);
                value.hash(state);
            }
            Value::Float(value) => {
                3u8.hash(state);
                value.to_bits().hash(state);
            }
            Value::Str(value) => {
                4u8.hash(state);
                value.hash(state);
            }
            Value::List(value) => {
                5u8.hash(state);
                Rc::as_ptr(value).hash(state);
            }
            Value::Map(value) => {
                6u8.hash(state);
                Rc::as_ptr(value).hash(state);
            }
            Value::Function(value) => {
                7u8.hash(state);
                Rc::as_ptr(value).hash(state);
            }
            Value::Native(value) => {
                8u8.hash(state);
                (*value as *const NativeFunction).hash(state);
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Str(Rc::from(value))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Str(Rc::from(value.as_str()))
    }
}

/// Renders a value, guarding against cycles in self-referential lists.
fn write_value(
    f: &mut fmt::Formatter<'_>,
    value: &Value,
    depth: usize,
    quoted: bool,
) -> fmt::Result {
    const MAX_DEPTH: usize = 4;
    match value {
        Value::Nil => f.write_str("nil"),
        Value::Bool(inner) => write!(f, "{inner}"),
        Value::Int(inner) => write!(f, "{inner}"),
        Value::Float(inner) => {
            if inner.is_finite() && inner.fract() == 0.0 {
                write!(f, "{inner:.1}")
            } else {
                write!(f, "{inner}")
            }
        }
        Value::Str(inner) => {
            if quoted {
                write!(f, "{inner:?}")
            } else {
                f.write_str(inner)
            }
        }
        Value::List(items) => {
            if depth >= MAX_DEPTH {
                return f.write_str("[...]");
            }
            f.write_str("[")?;
            for (index, item) in items.borrow().iter().enumerate() {
                if index > 0 {
                    f.write_str(", ")?;
                }
                write_value(f, item, depth + 1, true)?;
            }
            f.write_str("]")
        }
        Value::Map(entries) => {
            if depth >= MAX_DEPTH {
                return f.write_str("#{...}");
            }
            f.write_str("#{")?;
            for (index, (key, item)) in entries.borrow().iter().enumerate() {
                if index > 0 {
                    f.write_str(", ")?;
                }
                write_value(f, &key.0, depth + 1, true)?;
                f.write_str(": ")?;
                write_value(f, item, depth + 1, true)?;
            }
            f.write_str("}")
        }
        Value::Function(function) => {
            write!(
                f,
                "<fn {}>",
                function.name.as_deref().unwrap_or("anonymous")
            )
        }
        Value::Native(function) => write!(f, "<native fn {}>", function.name),
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value(f, self, 0, false)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value(f, self, 0, true)
    }
}

/// A map key: a [`Value`] with a reflexive `Eq`/`Hash` implementation.
#[derive(Clone)]
pub struct MapKey(pub Value);

impl MapKey {
    /// Borrows the underlying value.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.0
    }
}

impl PartialEq for MapKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.key_eq(&other.0)
    }
}

impl Eq for MapKey {}

impl Hash for MapKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.key_hash(state);
    }
}

impl From<Value> for MapKey {
    fn from(value: Value) -> Self {
        MapKey(value)
    }
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

/// `lhs + rhs` with number/string/list overloading.
pub fn add(lhs: &Value, rhs: &Value, span: Span) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => a
            .checked_add(*b)
            .map(Value::Int)
            .ok_or_else(|| overflow(span)),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Str(a), Value::Str(b)) => {
            let mut text = String::with_capacity(a.len() + b.len());
            text.push_str(a);
            text.push_str(b);
            Ok(Value::string(text))
        }
        (Value::List(a), Value::List(b)) => {
            let mut items = a.borrow().clone();
            items.extend(b.borrow().iter().cloned());
            Ok(Value::list(items))
        }
        _ => Err(TypeError::binary("+", lhs, rhs).into_error(span)),
    }
}

/// `lhs - rhs`.
pub fn sub(lhs: &Value, rhs: &Value, span: Span) -> Result<Value, RuntimeError> {
    numeric(lhs, rhs, span, "-", i64::checked_sub, |a, b| a - b)
}

/// `lhs * rhs`.
pub fn mul(lhs: &Value, rhs: &Value, span: Span) -> Result<Value, RuntimeError> {
    numeric(lhs, rhs, span, "*", i64::checked_mul, |a, b| a * b)
}

/// `lhs / rhs`; division by zero is a runtime error.
pub fn div(lhs: &Value, rhs: &Value, span: Span) -> Result<Value, RuntimeError> {
    if let (Value::Int(a), Value::Int(b)) = (lhs, rhs) {
        if *b == 0 {
            return Err(RuntimeError::new(span, "division by zero"));
        }
        return a
            .checked_div(*b)
            .map(Value::Int)
            .ok_or_else(|| overflow(span));
    }
    numeric(lhs, rhs, span, "/", i64::checked_div, |a, b| a / b)
}

/// `lhs % rhs`.
pub fn rem(lhs: &Value, rhs: &Value, span: Span) -> Result<Value, RuntimeError> {
    if let (Value::Int(a), Value::Int(b)) = (lhs, rhs) {
        if *b == 0 {
            return Err(RuntimeError::new(span, "remainder by zero"));
        }
        return a
            .checked_rem(*b)
            .map(Value::Int)
            .ok_or_else(|| overflow(span));
    }
    numeric(lhs, rhs, span, "%", i64::checked_rem, |a, b| a % b)
}

/// `-value`.
pub fn negate(value: &Value, span: Span) -> Result<Value, RuntimeError> {
    match value {
        Value::Int(inner) => inner
            .checked_neg()
            .map(Value::Int)
            .ok_or_else(|| overflow(span)),
        Value::Float(inner) => Ok(Value::Float(-inner)),
        other => Err(RuntimeError::new(
            span,
            format!("cannot negate {}", other.type_name()),
        )),
    }
}

/// `<`, `<=`, `>`, `>=` for numbers and strings.
pub fn compare(lhs: &Value, rhs: &Value, span: Span) -> Result<core::cmp::Ordering, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b)),
        (Value::Str(a), Value::Str(b)) => Ok(a.cmp(b)),
        (a, b) if is_number(a) && is_number(b) => {
            let a = as_f64(a);
            let b = as_f64(b);
            a.partial_cmp(&b)
                .ok_or_else(|| RuntimeError::new(span, "cannot compare with NaN"))
        }
        _ => Err(RuntimeError::new(
            span,
            format!("cannot compare {} and {}", lhs.type_name(), rhs.type_name()),
        )),
    }
}

fn is_number(value: &Value) -> bool {
    matches!(value, Value::Int(_) | Value::Float(_))
}

fn as_f64(value: &Value) -> f64 {
    match value {
        Value::Int(inner) => *inner as f64,
        Value::Float(inner) => *inner,
        _ => f64::NAN,
    }
}

fn numeric(
    lhs: &Value,
    rhs: &Value,
    span: Span,
    op: &str,
    int_op: fn(i64, i64) -> Option<i64>,
    float_op: fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => {
            int_op(*a, *b).map(Value::Int).ok_or_else(|| overflow(span))
        }
        (a, b) if is_number(a) && is_number(b) => Ok(Value::Float(float_op(as_f64(a), as_f64(b)))),
        _ => Err(TypeError::binary(op, lhs, rhs).into_error(span)),
    }
}

fn overflow(span: Span) -> RuntimeError {
    RuntimeError::new(span, "integer overflow")
}

/// Describes a type mismatch, with a note suggesting a conversion.
struct TypeError {
    op: String,
    lhs: String,
    rhs: String,
}

impl TypeError {
    fn binary(op: &str, lhs: &Value, rhs: &Value) -> Self {
        Self {
            op: op.to_string(),
            lhs: lhs.type_name().to_string(),
            rhs: rhs.type_name().to_string(),
        }
    }

    fn into_error(self, span: Span) -> RuntimeError {
        RuntimeError::new(
            span,
            format!(
                "cannot apply `{}` to {} and {}",
                self.op, self.lhs, self.rhs
            ),
        )
        .with_note("operands of arithmetic operators must both be numbers")
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// One frame of a runtime stack trace.
#[derive(Clone, Debug)]
pub struct TraceFrame {
    pub function: String,
    pub span: Span,
}

/// A runtime error: a diagnostic plus the call stack that produced it.
#[derive(Clone, Debug)]
pub struct RuntimeError {
    pub diagnostic: Box<Diagnostic>,
    pub trace: Vec<TraceFrame>,
}

impl RuntimeError {
    /// Creates an error at `span`.
    #[must_use]
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            diagnostic: Box::new(
                Diagnostic::error(message)
                    .with_code("E2001")
                    .with_span(span),
            ),
            trace: Vec::new(),
        }
    }

    /// Attaches a note to the diagnostic.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        let diagnostic = *self.diagnostic;
        self.diagnostic = Box::new(diagnostic.with_note(note));
        self
    }

    /// Pushes a stack frame.
    #[must_use]
    pub fn with_frame(mut self, function: impl Into<String>, span: Span) -> Self {
        self.trace.push(TraceFrame {
            function: function.into(),
            span,
        });
        self
    }

    /// The message text.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.diagnostic.message
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &*self.diagnostic)
    }
}

impl std::error::Error for RuntimeError {}

// ---------------------------------------------------------------------------
// Environments
// ---------------------------------------------------------------------------

/// A lexical scope: a set of names plus a link to the enclosing scope.
pub struct Env {
    values: HashMap<String, Value>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    /// An empty global scope.
    #[must_use]
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            parent: None,
        }
    }

    /// A scope nested inside `parent`.
    #[must_use]
    pub fn child(parent: Rc<RefCell<Env>>) -> Rc<RefCell<Env>> {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: Some(parent),
        }))
    }

    /// Binds `name` in this scope, shadowing any outer binding.
    pub fn define(&mut self, name: impl Into<String>, value: Value) {
        self.values.insert(name.into(), value);
    }

    /// Looks up `name`, walking outward through parents.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.values.get(name) {
            return Some(value.clone());
        }
        self.parent.as_ref()?.borrow().get(name)
    }

    /// Assigns to an existing binding, walking outward. Returns whether it was
    /// found; [`false`] means the name is undefined.
    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        if let Some(slot) = self.values.get_mut(name) {
            *slot = value;
            return true;
        }
        match &self.parent {
            Some(parent) => parent.borrow_mut().assign(name, value),
            None => false,
        }
    }

    /// Whether this exact scope (not its parents) defines `name`.
    #[must_use]
    pub fn contains_local(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    /// Number of names defined in this scope.
    #[must_use]
    pub fn local_len(&self) -> usize {
        self.values.len()
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Env")
            .field("locals", &self.values.len())
            .field("has_parent", &self.parent.is_some())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// A user-defined function or closure.
pub struct Function {
    /// `Some` for `fn name(..)`, `None` for an anonymous closure.
    pub name: Option<String>,
    /// Parameter names, in order.
    pub params: Vec<String>,
    /// The body statements.
    pub body: Vec<Stmt>,
    /// The environment the function captured when it was created.
    pub env: Rc<RefCell<Env>>,
}

impl Function {
    /// Builds a function value.
    #[must_use]
    pub fn new(
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
        env: Rc<RefCell<Env>>,
    ) -> Self {
        Self {
            name,
            params,
            body,
            env,
        }
    }

    /// How many parameters the function takes.
    #[must_use]
    pub fn arity(&self) -> usize {
        self.params.len()
    }

    /// A display name for stack traces.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("<anonymous>")
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Function")
            .field("name", &self.name)
            .field("params", &self.params)
            .field("body_len", &self.body.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Native functions
// ---------------------------------------------------------------------------

/// How many arguments a native function accepts.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Arity {
    /// Exactly this many.
    Exact(usize),
    /// At least this many.
    AtLeast(usize),
    /// Any number.
    Any,
}

impl Arity {
    /// Whether `count` arguments are acceptable.
    #[must_use]
    pub const fn accepts(self, count: usize) -> bool {
        match self {
            Arity::Exact(n) => count == n,
            Arity::AtLeast(n) => count >= n,
            Arity::Any => true,
        }
    }

    /// A human description used in arity errors.
    #[must_use]
    pub fn describe(self) -> String {
        match self {
            Arity::Exact(n) => format!("exactly {n}"),
            Arity::AtLeast(n) => format!("at least {n}"),
            Arity::Any => "any number of".to_string(),
        }
    }
}

/// Where a native function writes its output.
///
/// Making this a trait means tests can capture output instead of printing to
/// the terminal, and it is the reason the interpreter never calls `println!`.
pub trait Output {
    /// Writes one line.
    fn write_line(&mut self, line: &str);
}

/// Writes to standard output.
#[derive(Debug, Default)]
pub struct Stdout;

impl Output for Stdout {
    fn write_line(&mut self, line: &str) {
        println!("{line}");
    }
}

/// Collects output in memory; convenient for tests.
#[derive(Debug, Default)]
pub struct Buffer {
    lines: Vec<String>,
}

impl Buffer {
    /// A fresh buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The captured lines.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The captured output as a single newline-joined string.
    #[must_use]
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }
}

impl Output for Buffer {
    fn write_line(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }
}

/// The signature every builtin shares.
pub type NativeImpl = fn(&mut dyn Output, Span, &[Value]) -> Result<Value, RuntimeError>;

/// A builtin function.
pub struct NativeFunction {
    /// Name as seen from Pebble.
    pub name: &'static str,
    /// Accepted argument count.
    pub arity: Arity,
    /// The implementation.
    pub func: NativeImpl,
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native {}>", self.name)
    }
}

impl NativeFunction {
    /// The builtin's name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    fn call(
        &self,
        out: &mut dyn Output,
        span: Span,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        if !self.arity.accepts(args.len()) {
            return Err(RuntimeError::new(
                span,
                format!(
                    "{} expects {} arguments, got {}",
                    self.name,
                    self.arity.describe(),
                    args.len()
                ),
            ));
        }
        (self.func)(out, span, args)
    }
}

fn builtin_print(out: &mut dyn Output, _: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    let text = args
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    out.write_line(&text);
    Ok(Value::Nil)
}

fn builtin_len(_: &mut dyn Output, span: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    let len = match &args[0] {
        Value::Str(text) => text.chars().count(),
        Value::List(items) => items.borrow().len(),
        Value::Map(entries) => entries.borrow().len(),
        other => {
            return Err(RuntimeError::new(
                span,
                format!("`len` does not accept {}", other.type_name()),
            ));
        }
    };
    Ok(Value::Int(len as i64))
}

fn builtin_type(_: &mut dyn Output, _: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::string(args[0].type_name()))
}

fn builtin_str(_: &mut dyn Output, _: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::string(args[0].to_string()))
}

fn builtin_int(_: &mut dyn Output, span: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    Ok(match &args[0] {
        Value::Int(value) => Value::Int(*value),
        Value::Float(value) => Value::Int(*value as i64),
        Value::Bool(value) => Value::Int(i64::from(u8::from(*value))),
        Value::Str(text) => {
            text.trim().parse::<i64>().map(Value::Int).map_err(|_| {
                RuntimeError::new(span, format!("cannot parse {text:?} as an integer"))
            })?
        }
        other => {
            return Err(RuntimeError::new(
                span,
                format!("cannot convert {} to int", other.type_name()),
            ));
        }
    })
}

fn builtin_float(_: &mut dyn Output, span: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    Ok(match &args[0] {
        Value::Int(value) => Value::Float(*value as f64),
        Value::Float(value) => Value::Float(*value),
        Value::Str(text) => text
            .trim()
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| RuntimeError::new(span, format!("cannot parse {text:?} as a float")))?,
        other => {
            return Err(RuntimeError::new(
                span,
                format!("cannot convert {} to float", other.type_name()),
            ));
        }
    })
}

fn builtin_range(_: &mut dyn Output, span: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    let (start, end) = match args {
        [end] => (0, int_arg(end, span)?),
        [start, end] => (int_arg(start, span)?, int_arg(end, span)?),
        _ => unreachable!("arity checked by caller"),
    };
    let items: Vec<Value> = (start..end).map(Value::Int).collect();
    Ok(Value::list(items))
}

fn builtin_push(_: &mut dyn Output, span: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    let Value::List(items) = &args[0] else {
        return Err(RuntimeError::new(
            span,
            format!("`push` expects a list, got {}", args[0].type_name()),
        ));
    };
    items.borrow_mut().push(args[1].clone());
    Ok(Value::Nil)
}

fn builtin_keys(_: &mut dyn Output, span: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    let Value::Map(entries) = &args[0] else {
        return Err(RuntimeError::new(
            span,
            format!("`keys` expects a map, got {}", args[0].type_name()),
        ));
    };
    let keys = entries
        .borrow()
        .keys()
        .map(|key| key.0.clone())
        .collect::<Vec<_>>();
    Ok(Value::list(keys))
}

fn builtin_clock(_: &mut dyn Output, _: Span, _: &[Value]) -> Result<Value, RuntimeError> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs_f64())
        .unwrap_or(0.0);
    Ok(Value::Float(seconds))
}

fn builtin_assert(out: &mut dyn Output, span: Span, args: &[Value]) -> Result<Value, RuntimeError> {
    if args[0].is_truthy() {
        return Ok(Value::Nil);
    }
    let message = args
        .get(1)
        .map_or_else(|| "assertion failed".to_string(), ToString::to_string);
    // Route the message through `Output` so failures show up in captured logs.
    out.write_line(&message);
    Err(RuntimeError::new(span, message))
}

fn int_arg(value: &Value, span: Span) -> Result<i64, RuntimeError> {
    match value {
        Value::Int(inner) => Ok(*inner),
        other => Err(RuntimeError::new(
            span,
            format!("expected an integer, got {}", other.type_name()),
        )),
    }
}

/// The builtin functions, in one place.
pub static BUILTINS: &[NativeFunction] = &[
    NativeFunction {
        name: "print",
        arity: Arity::Any,
        func: builtin_print,
    },
    NativeFunction {
        name: "len",
        arity: Arity::Exact(1),
        func: builtin_len,
    },
    NativeFunction {
        name: "type",
        arity: Arity::Exact(1),
        func: builtin_type,
    },
    NativeFunction {
        name: "str",
        arity: Arity::Exact(1),
        func: builtin_str,
    },
    NativeFunction {
        name: "int",
        arity: Arity::Exact(1),
        func: builtin_int,
    },
    NativeFunction {
        name: "float",
        arity: Arity::Exact(1),
        func: builtin_float,
    },
    NativeFunction {
        name: "range",
        arity: Arity::AtLeast(1),
        func: builtin_range,
    },
    NativeFunction {
        name: "push",
        arity: Arity::Exact(2),
        func: builtin_push,
    },
    NativeFunction {
        name: "keys",
        arity: Arity::Exact(1),
        func: builtin_keys,
    },
    NativeFunction {
        name: "clock",
        arity: Arity::Exact(0),
        func: builtin_clock,
    },
    NativeFunction {
        name: "assert",
        arity: Arity::AtLeast(1),
        func: builtin_assert,
    },
];

/// Looks up a builtin by name.
#[must_use]
pub fn builtin(name: &str) -> Option<&'static NativeFunction> {
    BUILTINS.iter().find(|function| function.name == name)
}

/// The names of every builtin, in table order.
pub fn builtin_names() -> impl Iterator<Item = &'static str> {
    BUILTINS.iter().map(|function| function.name)
}

/// Calls a native function with a runtime arity check.
pub fn call_native(
    function: &'static NativeFunction,
    out: &mut dyn Output,
    span: Span,
    args: &[Value],
) -> Result<Value, RuntimeError> {
    function.call(out, span, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span::DUMMY
    }

    #[test]
    fn numbers_add_across_variants() {
        assert_eq!(
            add(&Value::Int(1), &Value::Int(2), span()).unwrap(),
            Value::Int(3)
        );
        assert_eq!(
            add(&Value::Int(1), &Value::Float(0.5), span()).unwrap(),
            Value::Float(1.5)
        );
    }

    #[test]
    fn integer_overflow_is_reported_not_wrapped() {
        let error = add(&Value::Int(i64::MAX), &Value::Int(1), span()).unwrap_err();
        assert_eq!(error.message(), "integer overflow");
    }

    #[test]
    fn strings_and_lists_overload_plus() {
        let hello = Value::string("he");
        let llo = Value::string("llo");
        assert_eq!(add(&hello, &llo, span()).unwrap().to_string(), "hello");

        let a = Value::list(vec![Value::Int(1)]);
        let b = Value::list(vec![Value::Int(2)]);
        assert_eq!(add(&a, &b, span()).unwrap().to_string(), "[1, 2]");
    }

    #[test]
    fn type_errors_are_clear() {
        let error = sub(&Value::string("a"), &Value::Int(1), span()).unwrap_err();
        assert!(
            error
                .message()
                .contains("cannot apply `-` to string and int")
        );
    }

    #[test]
    fn division_by_zero_is_an_error() {
        assert!(div(&Value::Int(1), &Value::Int(0), span()).is_err());
        // Floating point division by zero is IEEE, not an error.
        assert!(div(&Value::Float(1.0), &Value::Int(0), span()).is_ok());
    }

    #[test]
    fn list_identity_not_structure_for_equality() {
        let a = Value::list(vec![Value::Int(1)]);
        let b = Value::list(vec![Value::Int(1)]);
        assert!(!a.equals(&b));
        assert!(a.equals(&a.clone()));
    }

    #[test]
    fn map_keys_survive_nan_and_lists() {
        let mut map = HashMap::new();
        map.insert(MapKey(Value::Float(f64::NAN)), Value::Int(1));
        map.insert(MapKey(Value::list(vec![])), Value::Int(2));
        // A fresh NaN key must find the stored entry thanks to bitwise hashing.
        assert_eq!(
            map.get(&MapKey(Value::Float(f64::NAN))),
            Some(&Value::Int(1))
        );
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn environments_shadow_and_assign_outward() {
        let global = Rc::new(RefCell::new(Env::new()));
        global.borrow_mut().define("x", Value::Int(1));
        let inner = Env::child(Rc::clone(&global));
        inner.borrow_mut().define("x", Value::Int(2));
        assert_eq!(inner.borrow().get("x").unwrap(), Value::Int(2));
        assert!(inner.borrow_mut().assign("x", Value::Int(3)));
        // Assignment inside the child updates the child's own binding.
        assert_eq!(inner.borrow().get("x").unwrap(), Value::Int(3));

        let other = Env::child(Rc::clone(&global));
        assert!(other.borrow_mut().assign("x", Value::Int(9)));
        assert_eq!(global.borrow().get("x").unwrap(), Value::Int(9));
    }

    #[test]
    fn builtin_names_follow_the_table() {
        assert_eq!(
            builtin_names().collect::<Vec<_>>(),
            [
                "print", "len", "type", "str", "int", "float", "range", "push", "keys", "clock",
                "assert"
            ]
        );
    }

    #[test]
    fn builtins_are_looked_up_and_arity_checked() {
        let print = builtin("print").unwrap();
        let mut buffer = Buffer::new();
        call_native(print, &mut buffer, span(), &[Value::Int(1), Value::Int(2)]).unwrap();
        assert_eq!(buffer.text(), "1 2");

        let len = builtin("len").unwrap();
        let error = call_native(len, &mut buffer, span(), &[]).unwrap_err();
        assert!(error.message().contains("expects exactly 1"));
    }

    #[test]
    fn range_and_push_work() {
        let mut buffer = Buffer::new();
        let range = builtin("range").unwrap();
        let list = call_native(range, &mut buffer, span(), &[Value::Int(3)]).unwrap();
        assert_eq!(list.to_string(), "[0, 1, 2]");

        let push = builtin("push").unwrap();
        call_native(push, &mut buffer, span(), &[list.clone(), Value::Int(9)]).unwrap();
        assert_eq!(list.to_string(), "[0, 1, 2, 9]");
    }
}
