//! One trait over both execution engines.
//!
//! `pebble run` and `pebble bench` do the same three things with either
//! engine: turn the AST into something executable, build a fresh engine, run
//! it. [`Backend`] names those three steps so the command code is written once
//! and instantiated per engine (static dispatch, no `dyn`).

use pebble_ast::Program;
use pebble_bytecode::{Compiled, compile};
use pebble_diag::Diagnostic;
use pebble_interp::Interpreter;
use pebble_value::{Output, RuntimeError, Value};
use pebble_vm::{Vm, VmValue};

/// An execution engine for Pebble programs.
pub(crate) trait Backend {
    /// Label used by `pebble bench`.
    const NAME: &'static str;

    /// What the engine executes: the borrowed AST for the tree-walker, owned
    /// bytecode for the VM.
    type Prepared<'p>;

    /// Turns a parsed program into the engine's input, or reports why it cannot.
    fn prepare(program: &Program) -> Result<Self::Prepared<'_>, Vec<Diagnostic>>;

    /// Builds a fresh engine whose `print` goes to `out`.
    fn with_output(out: Box<dyn Output>) -> Self;

    /// Runs the program; `Ok(None)` when its final value is `nil`.
    fn execute(&mut self, prepared: &Self::Prepared<'_>) -> Result<Option<String>, RuntimeError>;
}

/// The tree-walking interpreter.
pub(crate) struct TreeWalker(Interpreter);

impl Backend for TreeWalker {
    const NAME: &'static str = "tree-walker";
    type Prepared<'p> = &'p Program;

    fn prepare(program: &Program) -> Result<&Program, Vec<Diagnostic>> {
        Ok(program)
    }

    fn with_output(out: Box<dyn Output>) -> Self {
        Self(Interpreter::with_output(out))
    }

    fn execute(&mut self, prepared: &&Program) -> Result<Option<String>, RuntimeError> {
        let value = self.0.interpret(prepared)?;
        // A script usually ends in a statement, so its value is `nil`;
        // printing that for every program would just be noise.
        Ok((!matches!(value, Value::Nil)).then(|| value.to_string()))
    }
}

/// The bytecode compiler plus the stack VM.
pub(crate) struct BytecodeVm(Vm);

impl Backend for BytecodeVm {
    const NAME: &'static str = "vm";
    type Prepared<'p> = Compiled;

    fn prepare(program: &Program) -> Result<Compiled, Vec<Diagnostic>> {
        let result = compile(program);
        if result.diagnostics.is_empty() {
            Ok(result.compiled)
        } else {
            Err(result.diagnostics)
        }
    }

    fn with_output(out: Box<dyn Output>) -> Self {
        Self(Vm::with_output(out))
    }

    fn execute(&mut self, prepared: &Compiled) -> Result<Option<String>, RuntimeError> {
        let value = self.0.run(prepared)?;
        Ok((value != VmValue::Nil).then(|| self.0.display(value)))
    }
}
