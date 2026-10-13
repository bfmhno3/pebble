#![forbid(unsafe_code)]
// Map literals build `HashMap<MapKey, Value>`; see the note in `pebble-value`
// for why that key type is sound despite the interior mutability.
#![allow(clippy::mutable_key_type)]
//! A tree-walking interpreter for Pebble.
//!
//! The interpreter is the simplest backend: it walks the [`pebble_ast`] and
//! evaluates each node. It is also the clearest demonstration of Rust's
//! ownership model in the whole workspace, because
//!
//! * environments are `Rc<RefCell<Env>>` — shared, mutable scope chains, and
//! * control flow is **data**: [`Flow`] is a value returned up the call stack,
//!   which is how `return`, `break`, and `continue` cross expression
//!   boundaries without exceptions or `goto`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use pebble_ast::{BinaryOp, Expr, LogicalOp, Program, Spanned, Stmt, UnaryOp};
use pebble_span::Span;
use pebble_value::{
    Env, Function, MapKey, Output, RuntimeError, Stdout, Value, builtin, call_native, compare, div,
    mul, negate, rem, sub,
};

/// How a statement finished.
///
/// Most statements produce a value and fall through (`Value`); the other three
/// variants unwind the walker.
#[derive(Debug)]
pub enum Flow {
    /// Completed normally, producing this value.
    Value(Value),
    /// A `return` escaped.
    Return(Value),
    /// A `break` escaped.
    Break,
    /// A `continue` escaped.
    Continue,
}

impl Flow {
    fn is_normal(&self) -> bool {
        matches!(self, Flow::Value(_))
    }
}

/// The tree-walking interpreter.
pub struct Interpreter {
    globals: Rc<RefCell<Env>>,
    out: Box<dyn Output>,
    depth: usize,
    max_depth: usize,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    /// Creates an interpreter that prints to standard output.
    #[must_use]
    pub fn new() -> Self {
        Self::with_output(Box::new(Stdout))
    }

    /// Creates an interpreter with a custom output sink.
    #[must_use]
    pub fn with_output(out: Box<dyn Output>) -> Self {
        let globals = Rc::new(RefCell::new(Env::new()));
        {
            let mut scope = globals.borrow_mut();
            for function in pebble_value::BUILTINS {
                scope.define(function.name, Value::Native(function));
            }
        }
        Self {
            globals,
            out,
            depth: 0,
            // Kept well below what the host stack can take, so a runaway
            // recursion produces a clean Pebble error instead of a SIGSEGV.
            // Bump it with `set_max_depth` if you need deeper recursion.
            max_depth: 128,
        }
    }

    /// The global scope.
    #[must_use]
    pub fn globals(&self) -> Rc<RefCell<Env>> {
        Rc::clone(&self.globals)
    }

    /// The current recursion limit.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Sets the recursion limit.
    pub fn set_max_depth(&mut self, depth: usize) {
        self.max_depth = depth;
    }

    /// Defines a value in the global scope from Rust.
    pub fn define_global(&mut self, name: impl Into<String>, value: Value) {
        self.globals.borrow_mut().define(name, value);
    }

    /// Evaluates a whole program, returning the value of the last statement.
    pub fn interpret(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        self.depth = 0;
        let globals = Rc::clone(&self.globals);
        match self.execute_block(&program.stmts, globals)? {
            Flow::Value(value) | Flow::Return(value) => Ok(value),
            Flow::Break | Flow::Continue => Err(RuntimeError::new(
                Span::DUMMY,
                "`break` or `continue` used outside a loop",
            )),
        }
    }

    /// Calls a function value with the given arguments.
    pub fn call(
        &mut self,
        callee: Value,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        self.call_value(callee, args, span)
    }

    // --- statements -------------------------------------------------------

    fn execute_block(
        &mut self,
        stmts: &[Stmt],
        env: Rc<RefCell<Env>>,
    ) -> Result<Flow, RuntimeError> {
        let mut last = Flow::Value(Value::Nil);
        for stmt in stmts {
            last = self.execute_stmt(stmt, &env)?;
            if !last.is_normal() {
                return Ok(last);
            }
        }
        Ok(last)
    }

    /// Runs a nested block in a fresh child scope, normalising its value to
    /// `nil`.
    ///
    /// Only expression statements carry a value; a control-flow statement such
    /// as `if` produces `nil`. The bytecode compiler makes the same choice, so
    /// the two backends agree on what a program evaluates to.
    fn run_scoped_block(
        &mut self,
        stmts: &[Stmt],
        env: &Rc<RefCell<Env>>,
    ) -> Result<Flow, RuntimeError> {
        let scope = Env::child(Rc::clone(env));
        match self.execute_block(stmts, scope)? {
            Flow::Value(_) => Ok(Flow::Value(Value::Nil)),
            other => Ok(other),
        }
    }

    fn execute_stmt(&mut self, stmt: &Stmt, env: &Rc<RefCell<Env>>) -> Result<Flow, RuntimeError> {
        match stmt {
            Stmt::Let { name, value, .. } => {
                let value = self.eval(value, env)?;
                env.borrow_mut().define(name.name.clone(), value);
                Ok(Flow::Value(Value::Nil))
            }
            Stmt::Fn {
                name, params, body, ..
            } => {
                let function = Function::new(
                    Some(name.name.clone()),
                    params.iter().map(|p| p.name.clone()).collect(),
                    body.clone(),
                    Rc::clone(env),
                );
                env.borrow_mut()
                    .define(name.name.clone(), Value::Function(Rc::new(function)));
                Ok(Flow::Value(Value::Nil))
            }
            Stmt::Expr { expr, .. } => Ok(Flow::Value(self.eval(expr, env)?)),
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                if self.eval(cond, env)?.is_truthy() {
                    self.run_scoped_block(then_branch, env)
                } else if else_branch.is_empty() {
                    Ok(Flow::Value(Value::Nil))
                } else {
                    self.run_scoped_block(else_branch, env)
                }
            }
            Stmt::While { cond, body, .. } => {
                while self.eval(cond, env)?.is_truthy() {
                    let scope = Env::child(Rc::clone(env));
                    match self.execute_block(body, scope)? {
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                        Flow::Break => break,
                        Flow::Value(_) | Flow::Continue => {}
                    }
                }
                Ok(Flow::Value(Value::Nil))
            }
            Stmt::For {
                var, iter, body, ..
            } => {
                let iterable = self.eval(iter, env)?;
                let Value::List(items) = &iterable else {
                    return Err(RuntimeError::new(
                        iterable_span(iter),
                        format!("`for` needs a list, got {}", iterable.type_name()),
                    ));
                };
                let snapshot = items.borrow().clone();
                for item in snapshot {
                    let scope = Env::child(Rc::clone(env));
                    scope.borrow_mut().define(var.name.clone(), item);
                    match self.execute_block(body, scope)? {
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                        Flow::Break => break,
                        Flow::Value(_) | Flow::Continue => {}
                    }
                }
                Ok(Flow::Value(Value::Nil))
            }
            Stmt::Return { value, .. } => {
                let value = match value {
                    Some(expr) => self.eval(expr, env)?,
                    None => Value::Nil,
                };
                Ok(Flow::Return(value))
            }
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::Continue),
        }
    }

    // --- expressions ------------------------------------------------------

    fn eval(&mut self, expr: &Expr, env: &Rc<RefCell<Env>>) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Nil { .. } => Ok(Value::Nil),
            Expr::Bool { value, .. } => Ok(Value::Bool(*value)),
            Expr::Int { value, .. } => Ok(Value::Int(*value)),
            Expr::Float { value, .. } => Ok(Value::Float(*value)),
            Expr::Str { value, .. } => Ok(Value::string(value.clone())),
            Expr::Var { name, span } => env.borrow().get(&name.name).ok_or_else(|| {
                RuntimeError::new(*span, format!("undefined variable `{}`", name.name))
                    .with_note("declare it first with `let`")
            }),
            Expr::List { items, .. } => {
                let values = items
                    .iter()
                    .map(|item| self.eval(item, env))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::list(values))
            }
            Expr::Map { entries, .. } => {
                let mut map = HashMap::with_capacity(entries.len());
                for (key, value) in entries {
                    let key = self.eval(key, env)?;
                    let value = self.eval(value, env)?;
                    map.insert(MapKey(key), value);
                }
                Ok(Value::Map(Rc::new(RefCell::new(map))))
            }
            Expr::Unary { op, operand, span } => {
                let value = self.eval(operand, env)?;
                match op {
                    UnaryOp::Neg => negate(&value, *span),
                    UnaryOp::Not => Ok(Value::Bool(!value.is_truthy())),
                }
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let lhs = self.eval(lhs, env)?;
                let rhs = self.eval(rhs, env)?;
                eval_binary(*op, &lhs, &rhs, *span)
            }
            Expr::Logical { op, lhs, rhs, .. } => {
                let lhs = self.eval(lhs, env)?;
                match op {
                    LogicalOp::And => {
                        if lhs.is_truthy() {
                            self.eval(rhs, env)
                        } else {
                            Ok(lhs)
                        }
                    }
                    LogicalOp::Or => {
                        if lhs.is_truthy() {
                            Ok(lhs)
                        } else {
                            self.eval(rhs, env)
                        }
                    }
                }
            }
            Expr::Assign { target, value, .. } => {
                let new_value = self.eval(value, env)?;
                match target.as_ref() {
                    Expr::Var { name, span } => {
                        if env.borrow_mut().assign(&name.name, new_value.clone()) {
                            Ok(new_value)
                        } else {
                            Err(RuntimeError::new(
                                *span,
                                format!("cannot assign to undefined variable `{}`", name.name),
                            ))
                        }
                    }
                    Expr::Index {
                        object,
                        index,
                        span,
                    } => {
                        let container = self.eval(object, env)?;
                        let key = self.eval(index, env)?;
                        assign_index(&container, &key, new_value.clone(), *span)?;
                        Ok(new_value)
                    }
                    other => Err(RuntimeError::new(other.span(), "invalid assignment target")),
                }
            }
            Expr::Call { callee, args, span } => {
                let callee = self.eval(callee, env)?;
                let args = args
                    .iter()
                    .map(|arg| self.eval(arg, env))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_value(callee, args, *span)
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let container = self.eval(object, env)?;
                let key = self.eval(index, env)?;
                index_value(&container, &key, *span)
            }
            Expr::Fn { params, body, .. } => {
                let function = Function::new(
                    None,
                    params.iter().map(|p| p.name.clone()).collect(),
                    body.clone(),
                    Rc::clone(env),
                );
                Ok(Value::Function(Rc::new(function)))
            }
        }
    }

    // --- calls ------------------------------------------------------------

    fn call_value(
        &mut self,
        callee: Value,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match callee {
            Value::Native(function) => {
                let out = &mut *self.out;
                call_native(function, out, span, &args)
            }
            Value::Function(function) => {
                if args.len() != function.params.len() {
                    return Err(RuntimeError::new(
                        span,
                        format!(
                            "{} expects {} argument(s), got {}",
                            function.display_name(),
                            function.params.len(),
                            args.len()
                        ),
                    ));
                }
                let result = self.with_call_depth(span, |this| this.run_function(&function, args));
                result.map_err(|error| error.with_frame(function.display_name(), span))
            }
            other => Err(RuntimeError::new(
                span,
                format!("{} is not callable", other.type_name()),
            )),
        }
    }

    fn run_function(
        &mut self,
        function: &Rc<Function>,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let env = Env::child(Rc::clone(&function.env));
        {
            let mut scope = env.borrow_mut();
            for (param, arg) in function.params.iter().zip(args) {
                scope.define(param.clone(), arg);
            }
        }
        match self.execute_block(&function.body, env)? {
            Flow::Return(value) | Flow::Value(value) => Ok(value),
            Flow::Break | Flow::Continue => Err(RuntimeError::new(
                function.body.first().map_or(Span::DUMMY, Spanned::span),
                "`break` or `continue` escaped a function body",
            )),
        }
    }

    /// Runs `body` one call level deeper, failing first if that would exceed the limit.
    fn with_call_depth<T>(
        &mut self,
        span: Span,
        body: impl FnOnce(&mut Self) -> Result<T, RuntimeError>,
    ) -> Result<T, RuntimeError> {
        if self.depth >= self.max_depth {
            return Err(RuntimeError::new(
                span,
                format!("call stack overflow (recursion limit {})", self.max_depth),
            ));
        }
        self.depth += 1;
        let result = body(self);
        self.depth -= 1;
        result
    }
}

fn eval_binary(op: BinaryOp, lhs: &Value, rhs: &Value, span: Span) -> Result<Value, RuntimeError> {
    use std::cmp::Ordering;
    Ok(match op {
        BinaryOp::Add => pebble_value::add(lhs, rhs, span)?,
        BinaryOp::Sub => sub(lhs, rhs, span)?,
        BinaryOp::Mul => mul(lhs, rhs, span)?,
        BinaryOp::Div => div(lhs, rhs, span)?,
        BinaryOp::Rem => rem(lhs, rhs, span)?,
        BinaryOp::Eq => Value::Bool(lhs.equals(rhs)),
        BinaryOp::NotEq => Value::Bool(!lhs.equals(rhs)),
        BinaryOp::Lt => Value::Bool(compare(lhs, rhs, span)? == Ordering::Less),
        BinaryOp::LtEq => Value::Bool(compare(lhs, rhs, span)? != Ordering::Greater),
        BinaryOp::Gt => Value::Bool(compare(lhs, rhs, span)? == Ordering::Greater),
        BinaryOp::GtEq => Value::Bool(compare(lhs, rhs, span)? != Ordering::Less),
    })
}

fn index_value(container: &Value, key: &Value, span: Span) -> Result<Value, RuntimeError> {
    match container {
        Value::List(items) => {
            let index = resolve_index(&items.borrow(), key, span)?;
            Ok(items.borrow()[index].clone())
        }
        Value::Str(text) => {
            let chars: Vec<char> = text.chars().collect();
            let index = resolve_index(&chars, key, span)?;
            Ok(Value::string(chars[index].to_string()))
        }
        Value::Map(entries) => entries
            .borrow()
            .get(&MapKey(key.clone()))
            .cloned()
            .ok_or_else(|| RuntimeError::new(span, format!("key {key} is not in the map"))),
        other => Err(RuntimeError::new(
            span,
            format!("cannot index into {}", other.type_name()),
        )),
    }
}

fn assign_index(
    container: &Value,
    key: &Value,
    value: Value,
    span: Span,
) -> Result<(), RuntimeError> {
    match container {
        Value::List(items) => {
            let index = resolve_index(&items.borrow(), key, span)?;
            items.borrow_mut()[index] = value;
            Ok(())
        }
        Value::Map(entries) => {
            entries.borrow_mut().insert(MapKey(key.clone()), value);
            Ok(())
        }
        other => Err(RuntimeError::new(
            span,
            format!("cannot assign into {}", other.type_name()),
        )),
    }
}

/// Turns an index value into a `usize`, supporting negative indices.
fn resolve_index<T>(items: &[T], key: &Value, span: Span) -> Result<usize, RuntimeError> {
    let Value::Int(raw) = key else {
        return Err(RuntimeError::new(
            span,
            format!("list index must be an int, got {}", key.type_name()),
        ));
    };
    let len = items.len() as i64;
    let index = if *raw < 0 { len + raw } else { *raw };
    if index < 0 || index >= len {
        return Err(RuntimeError::new(
            span,
            format!("index {raw} is out of range for length {len}"),
        ));
    }
    Ok(index as usize)
}

fn iterable_span(expr: &Expr) -> Span {
    expr.span()
}

/// Convenience: looks up a builtin by name, for embedding tests and tools.
#[must_use]
pub fn builtin_value(name: &str) -> Option<Value> {
    builtin(name).map(Value::Native)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pebble_parser::parse;
    use pebble_span::SourceId;
    use pebble_value::Buffer;
    use std::rc::Weak;

    fn run(src: &str) -> Result<Value, RuntimeError> {
        let mut interpreter = Interpreter::with_output(Box::new(Buffer::new()));
        let result = parse(SourceId(0), src);
        assert!(result.is_ok(), "parse errors: {:?}", result.diagnostics);
        interpreter.interpret(&result.program)
    }

    fn run_with_output(src: &str) -> (Result<Value, RuntimeError>, Vec<String>) {
        struct Shared(Rc<RefCell<Buffer>>);
        impl Output for Shared {
            fn write_line(&mut self, line: &str) {
                self.0.borrow_mut().write_line(line);
            }
        }
        let shared = Rc::new(RefCell::new(Buffer::new()));
        let mut interpreter = Interpreter::with_output(Box::new(Shared(Rc::clone(&shared))));
        let result = parse(SourceId(0), src);
        let value = interpreter.interpret(&result.program);
        let lines = shared.borrow().lines().to_vec();
        (value, lines)
    }

    #[test]
    fn evaluates_arithmetic_with_precedence() {
        assert_eq!(run("1 + 2 * 3;").unwrap(), Value::Int(7));
        assert_eq!(run("(1 + 2) * 3;").unwrap(), Value::Int(9));
        assert_eq!(run("7 % 3;").unwrap(), Value::Int(1));
    }

    #[test]
    fn variables_and_assignment() {
        assert_eq!(run("let x = 1; x = x + 41; x;").unwrap(), Value::Int(42));
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
        assert_eq!(run(src).unwrap(), Value::Int(3));
    }

    #[test]
    fn recursion_works() {
        let src = "
            fn fact(n) {
                if n < 2 { return 1; }
                return n * fact(n - 1);
            }
            fact(10);
        ";
        assert_eq!(run(src).unwrap(), Value::Int(3_628_800));
    }

    #[test]
    fn lists_maps_and_indexing() {
        assert_eq!(
            run("let xs = [10, 20, 30]; xs[1];").unwrap(),
            Value::Int(20)
        );
        // Negative indices count from the end.
        assert_eq!(
            run("let xs = [10, 20, 30]; xs[-1];").unwrap(),
            Value::Int(30)
        );
        assert_eq!(
            run("let m = #{ \"a\": 1 }; m[\"a\"];").unwrap(),
            Value::Int(1)
        );
        assert_eq!(
            run("let xs = [1]; xs[0] = 5; xs[0];").unwrap(),
            Value::Int(5)
        );
    }

    #[test]
    fn string_concatenation_and_indexing() {
        assert_eq!(run("\"foo\" + \"bar\";").unwrap().to_string(), "foobar");
        assert_eq!(run("\"abc\"[2];").unwrap().to_string(), "c");
    }

    #[test]
    fn control_flow_loops() {
        let src = "
            let total = 0;
            for i in range(10) {
                if i == 3 { continue; }
                if i == 7 { break; }
                total = total + i;
            }
            total;
        ";
        // 0+1+2+4+5+6 = 18
        assert_eq!(run(src).unwrap(), Value::Int(18));
    }

    #[test]
    fn while_loop_accumulates() {
        let src = "let i = 0; let s = 0; while i < 5 { s = s + i; i = i + 1; } s;";
        assert_eq!(run(src).unwrap(), Value::Int(10));
    }

    #[test]
    fn print_reaches_the_output_sink() {
        let (result, lines) = run_with_output("print(\"hello\", 1 + 1);");
        assert!(result.is_ok());
        assert_eq!(lines, vec!["hello 2".to_string()]);
    }

    #[test]
    fn runtime_errors_are_reported_with_a_trace() {
        let error = run("fn boom() { return missing; } boom();").unwrap_err();
        assert!(error.message().contains("undefined variable `missing`"));
        assert_eq!(error.trace.len(), 1);
        assert_eq!(error.trace[0].function, "boom");
    }

    #[test]
    fn recursive_overflow_is_caught() {
        // Reaching the recursion limit needs far more host stack than a default
        // test thread has, and `Value` is not `Send`, so the assertion runs
        // inside a dedicated large-stack thread.
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let src = "fn loop() { return loop(); } loop();";
                let mut interpreter = Interpreter::with_output(Box::new(Buffer::new()));
                let result = parse(SourceId(0), src);
                let error = interpreter.interpret(&result.program).unwrap_err();
                assert!(error.message().contains("stack overflow"), "{error}");
            })
            .expect("spawn overflow thread")
            .join()
            .expect("overflow test panicked");
    }

    #[test]
    fn arity_errors_are_reported() {
        let error = run("fn f(a) { return a; } f();").unwrap_err();
        assert!(error.message().contains("expects 1 argument"));
    }

    #[test]
    fn calling_a_non_function_is_an_error() {
        let error = run("let x = 1; x();").unwrap_err();
        assert!(error.message().contains("not callable"));
    }

    #[test]
    fn type_errors_propagate_from_operators() {
        let error = run("1 + \"two\";").unwrap_err();
        assert!(error.message().contains("cannot apply `+`"));
    }

    /// Any function value stored in the environment it captured forms an `Rc`
    /// cycle (not only recursive ones): the globals own `fact`, and `fact`
    /// owns the globals it closed over.
    #[test]
    fn top_level_functions_keep_globals_alive() {
        let src = "fn fact(n) { if n < 2 { return 1; } return n * fact(n - 1); } fact(5);";
        let mut interpreter = Interpreter::with_output(Box::new(Buffer::new()));
        let result = parse(SourceId(0), src);
        interpreter.interpret(&result.program).unwrap();
        let globals: Weak<RefCell<Env>> = Rc::downgrade(&interpreter.globals());
        drop(interpreter);
        assert!(
            globals.upgrade().is_some(),
            "globals hold `fact`, `fact` holds globals: the cycle leaks"
        );
    }

    #[test]
    fn globals_without_functions_are_freed() {
        let mut interpreter = Interpreter::with_output(Box::new(Buffer::new()));
        let result = parse(SourceId(0), "let x = 1 + 2; x;");
        interpreter.interpret(&result.program).unwrap();
        let globals: Weak<RefCell<Env>> = Rc::downgrade(&interpreter.globals());
        drop(interpreter);
        assert!(globals.upgrade().is_none());
    }
}
