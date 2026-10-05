#![forbid(unsafe_code)]
//! The Pebble bytecode compiler: AST in, an instruction stream out.
//!
//! The compiler walks the tree in [`pebble_ast`] and emits a linear
//! [`Chunk`] of [`Opcode`]s. Three parts are worth studying:
//!
//! * The `opcode_set!` **function-like macro** from `pebble-macros` turns a
//!   plain list of opcodes into the enum plus a `name()` table.
//! * **Control flow** is compiled to jumps, and forward jumps are patched once
//!   their target is known ([`Chunk::patch_jump`]). This is the standard
//!   single-pass compiler trick.
//! * Variables are resolved **by name at run time**. That keeps closures simple
//!   (a closure captures its whole environment) at the cost of speed; the
//!   alternative, slot indices plus upvalues, is the optimisation a production
//!   compiler would make.

use std::fmt;
use std::rc::Rc;

use pebble_ast::{BinaryOp, Expr, LogicalOp, Program, Spanned, Stmt, UnaryOp};
use pebble_diag::Diagnostic;
use pebble_macros::opcode_set;
use pebble_span::Span;

opcode_set! {
    /// One Pebble instruction.
    ///
    /// Variants with a `u32` operand index into the chunk's constant pool, or
    /// name a jump target (an instruction index).
    pub enum Opcode {
        /// Push `constants[operand]`.
        Const(u32),
        /// Push `nil`.
        Nil,
        /// Push `true`.
        True,
        /// Push `false`.
        False,
        /// Discard the top of the stack.
        Pop,
        /// Define `constants[operand]` in the current frame.
        DefineName(u32),
        /// Push the value of `constants[operand]`.
        GetName(u32),
        /// Assign to the existing binding `constants[operand]`, leaving the
        /// value on the stack.
        SetName(u32),
        /// Build a closure from the function prototype `constants[operand]`,
        /// capturing the current frame.
        MakeClosure(u32),
        /// `a + b`
        Add,
        /// `a - b`
        Sub,
        /// `a * b`
        Mul,
        /// `a / b`
        Div,
        /// `a % b`
        Rem,
        /// `-a`
        Neg,
        /// `not a`
        Not,
        /// `a == b`
        Equal,
        /// `a != b`
        NotEqual,
        /// `a < b`
        Less,
        /// `a <= b`
        LessEqual,
        /// `a > b`
        Greater,
        /// `a >= b`
        GreaterEqual,
        /// Jump to an absolute instruction index.
        Jump(u32),
        /// Pop, then jump if the value was falsy.
        JumpIfFalse(u32),
        /// Peek: if falsy, jump keeping the value; otherwise pop and continue.
        JumpIfFalseKeep(u32),
        /// Peek: if truthy, jump keeping the value; otherwise pop and continue.
        JumpIfTrueKeep(u32),
        /// Call the callee sitting below `operand` arguments.
        Call(u32),
        /// Return the top of the stack.
        Return,
        /// Pop `operand` items into a list.
        BuildList(u32),
        /// Pop `operand` key/value pairs into a map.
        BuildMap(u32),
        /// `container[key]`
        Index,
        /// `container[key] = value`, leaving the value on the stack.
        SetIndex,
    }
}

impl Opcode {
    /// Number of bytes of inline operand this instruction would carry in a
    /// packed encoding. Pebble stores operands in the enum, but a real bytecode
    /// format flattens them into the instruction stream, so this is here to make
    /// that cost visible.
    #[must_use]
    pub const fn operand_bytes(self) -> usize {
        match self {
            Opcode::Const(_)
            | Opcode::DefineName(_)
            | Opcode::GetName(_)
            | Opcode::SetName(_)
            | Opcode::MakeClosure(_)
            | Opcode::Jump(_)
            | Opcode::JumpIfFalse(_)
            | Opcode::JumpIfFalseKeep(_)
            | Opcode::JumpIfTrueKeep(_)
            | Opcode::Call(_)
            | Opcode::BuildList(_)
            | Opcode::BuildMap(_) => 4,
            _ => 0,
        }
    }
}

/// A compile-time constant.
pub enum Constant {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    /// A nested function or closure body.
    Function(Rc<FunctionProto>),
}

impl Constant {
    fn same(&self, other: &Constant) -> bool {
        match (self, other) {
            (Constant::Nil, Constant::Nil) => true,
            (Constant::Bool(a), Constant::Bool(b)) => a == b,
            (Constant::Int(a), Constant::Int(b)) => a == b,
            (Constant::Float(a), Constant::Float(b)) => a.to_bits() == b.to_bits(),
            (Constant::Str(a), Constant::Str(b)) => a == b,
            // Function prototypes are never deduplicated.
            _ => false,
        }
    }
}

impl fmt::Debug for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Nil => f.write_str("nil"),
            Constant::Bool(value) => write!(f, "{value}"),
            Constant::Int(value) => write!(f, "{value}"),
            Constant::Float(value) => write!(f, "{value}"),
            Constant::Str(value) => write!(f, "{value:?}"),
            Constant::Function(proto) => write!(f, "<function {}>", proto.display_name()),
        }
    }
}

/// A compiled function body.
pub struct FunctionProto {
    /// `Some` for named declarations, `None` for anonymous closures.
    pub name: Option<String>,
    /// Parameter names, in order.
    pub params: Vec<String>,
    /// The function's instruction stream.
    pub chunk: Chunk,
}

impl FunctionProto {
    /// A display name for diagnostics.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("<anonymous>")
    }
}

impl fmt::Debug for FunctionProto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionProto")
            .field("name", &self.name)
            .field("params", &self.params)
            .field("instructions", &self.chunk.len())
            .finish()
    }
}

/// A linear sequence of instructions plus its constant pool.
#[derive(Default)]
pub struct Chunk {
    code: Vec<Opcode>,
    constants: Vec<Constant>,
    spans: Vec<Span>,
}

impl Chunk {
    /// An empty chunk.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an instruction and returns its index.
    pub fn emit(&mut self, op: Opcode, span: Span) -> usize {
        self.code.push(op);
        self.spans.push(span);
        self.code.len() - 1
    }

    /// Appends an instruction with no source span.
    ///
    /// Useful when building chunks programmatically; the VM only uses the span
    /// to attribute a runtime error, so a placeholder is fine.
    pub fn push(&mut self, op: Opcode) -> usize {
        self.emit(op, Span::DUMMY)
    }

    /// Appends a placeholder that will be patched once its target is known.
    fn placeholder(&mut self, op: Opcode, span: Span) -> usize {
        self.emit(op, span)
    }

    /// Points a previously emitted jump at `target`.
    ///
    /// # Panics
    /// Panics if `at` is not a jump instruction; that would be a compiler bug.
    pub fn patch_jump(&mut self, at: usize, target: usize) {
        let target = target as u32;
        self.code[at] = match self.code[at] {
            Opcode::Jump(_) => Opcode::Jump(target),
            Opcode::JumpIfFalse(_) => Opcode::JumpIfFalse(target),
            Opcode::JumpIfFalseKeep(_) => Opcode::JumpIfFalseKeep(target),
            Opcode::JumpIfTrueKeep(_) => Opcode::JumpIfTrueKeep(target),
            ref other => panic!("cannot patch {other:?} as a jump"),
        };
    }

    /// Adds a constant and returns its index.
    pub fn add_constant(&mut self, constant: Constant) -> u32 {
        self.constants.push(constant);
        (self.constants.len() - 1) as u32
    }

    /// The instructions.
    #[must_use]
    pub fn code(&self) -> &[Opcode] {
        &self.code
    }

    /// The constant pool.
    #[must_use]
    pub fn constants(&self) -> &[Constant] {
        &self.constants
    }

    /// The source span associated with instruction `index`.
    #[must_use]
    pub fn span_at(&self, index: usize) -> Span {
        self.spans.get(index).copied().unwrap_or(Span::DUMMY)
    }

    /// Number of instructions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Whether the chunk is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    /// A human-readable listing, `rustc`-style.
    #[must_use]
    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        self.disassemble_into(&mut out, 0);
        out
    }

    fn disassemble_into(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        out.push_str(&format!("{pad}== constants ==\n"));
        for (index, constant) in self.constants.iter().enumerate() {
            out.push_str(&format!("{pad}  {index:>4} {constant:?}\n"));
        }
        out.push_str(&format!("{pad}== code ==\n"));
        for (index, op) in self.code.iter().enumerate() {
            let operand = match op {
                Opcode::Const(index)
                | Opcode::DefineName(index)
                | Opcode::GetName(index)
                | Opcode::SetName(index)
                | Opcode::MakeClosure(index) => format!(" {index}"),
                Opcode::Jump(target)
                | Opcode::JumpIfFalse(target)
                | Opcode::JumpIfFalseKeep(target)
                | Opcode::JumpIfTrueKeep(target) => format!(" -> {target}"),
                Opcode::Call(count) | Opcode::BuildList(count) | Opcode::BuildMap(count) => {
                    format!(" {count}")
                }
                _ => String::new(),
            };
            out.push_str(&format!("{pad}  {index:>4} {}{operand}\n", op.name()));
            if let Opcode::MakeClosure(constant) = op
                && let Some(Constant::Function(proto)) = self.constants.get(*constant as usize)
            {
                out.push_str(&format!(
                    "{pad}      -- function {} --\n",
                    proto.display_name()
                ));
                proto.chunk.disassemble_into(out, indent + 3);
            }
        }
    }
}

impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("instructions", &self.code.len())
            .field("constants", &self.constants.len())
            .finish()
    }
}

/// Builds a [`Chunk`] from a list of opcodes.
///
/// This is the third flavour of macro in the project: a plain declarative
/// `macro_rules!`, next to the code that uses it rather than in the
/// `pebble-macros` crate.
///
/// ```
/// use pebble_bytecode::{chunk, Opcode};
///
/// let chunk = chunk![Opcode::Nil, Opcode::True, Opcode::Return];
/// assert_eq!(chunk.len(), 3);
/// ```
#[macro_export]
macro_rules! chunk {
    ($($op:expr),* $(,)?) => {{
        let mut chunk = $crate::Chunk::new();
        $(
            $crate::Chunk::push(&mut chunk, $op);
        )*
        chunk
    }};
}

/// The result of compiling a program.
pub struct Compiled {
    /// The top-level function. Its `chunk` is the program.
    pub main: Rc<FunctionProto>,
}

impl Compiled {
    /// The top-level instruction stream.
    #[must_use]
    pub fn chunk(&self) -> &Chunk {
        &self.main.chunk
    }
}

/// Compilation output: the program plus any diagnostics.
pub struct CompileResult {
    pub compiled: Compiled,
    pub diagnostics: Vec<Diagnostic>,
}

impl CompileResult {
    /// Whether compilation produced no diagnostics.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Compiles a parsed program.
#[must_use]
pub fn compile(program: &Program) -> CompileResult {
    let mut compiler = Compiler::new();
    compiler.compile_body(&program.stmts, false);
    CompileResult {
        compiled: Compiled {
            main: Rc::new(FunctionProto {
                name: Some("<main>".to_string()),
                params: Vec::new(),
                chunk: compiler.chunk,
            }),
        },
        diagnostics: compiler.diagnostics,
    }
}

/// Bookkeeping for one enclosing loop, so `break`/`continue` can be patched.
#[derive(Default)]
struct LoopCtx {
    continue_jumps: Vec<usize>,
    break_jumps: Vec<usize>,
}

struct Compiler {
    chunk: Chunk,
    loops: Vec<LoopCtx>,
    diagnostics: Vec<Diagnostic>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            loops: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn here(&self) -> usize {
        self.chunk.len()
    }

    fn emit(&mut self, op: Opcode, span: Span) -> usize {
        self.chunk.emit(op, span)
    }

    /// Interns a scalar constant, deduplicating identical ones.
    fn intern(&mut self, constant: Constant) -> u32 {
        if !matches!(constant, Constant::Function(_))
            && let Some(position) = self
                .chunk
                .constants
                .iter()
                .position(|existing| existing.same(&constant))
        {
            return position as u32;
        }
        self.chunk.add_constant(constant)
    }

    fn name_const(&mut self, name: &str) -> u32 {
        self.intern(Constant::Str(name.to_string()))
    }

    // --- statements -------------------------------------------------------

    /// Compiles a statement list. When `implicit_return` is set, a trailing
    /// expression becomes the block's value (via `Return`); otherwise the block
    /// leaves its value on the stack (top level) or nothing (nested).
    fn compile_body(&mut self, stmts: &[Stmt], in_function: bool) {
        let span = stmts.first().map_or(Span::DUMMY, Spanned::span);
        let Some((last, init)) = stmts.split_last() else {
            self.emit(Opcode::Nil, span);
            if in_function {
                self.emit(Opcode::Return, span);
            }
            return;
        };
        for stmt in init {
            self.compile_stmt(stmt);
        }
        if let Stmt::Expr { expr, .. } = last {
            self.compile_expr(expr);
        } else {
            self.compile_stmt(last);
            self.emit(Opcode::Nil, span);
        }
        if in_function {
            self.emit(Opcode::Return, span);
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) {
        let span = stmt.span();
        match stmt {
            Stmt::Let { name, value, .. } => {
                self.compile_expr(value);
                let index = self.name_const(&name.name);
                self.emit(Opcode::DefineName(index), span);
            }
            Stmt::Fn {
                name, params, body, ..
            } => {
                let index = self.compile_function(Some(name.name.clone()), params, body);
                self.emit(Opcode::MakeClosure(index), span);
                let name_index = self.name_const(&name.name);
                self.emit(Opcode::DefineName(name_index), span);
            }
            Stmt::Expr { expr, .. } => {
                self.compile_expr(expr);
                self.emit(Opcode::Pop, span);
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.compile_expr(cond);
                let jump_else = self.chunk.placeholder(Opcode::JumpIfFalse(0), span);
                for stmt in then_branch {
                    self.compile_stmt(stmt);
                }
                let jump_end = self.chunk.placeholder(Opcode::Jump(0), span);
                let else_target = self.here();
                self.chunk.patch_jump(jump_else, else_target);
                for stmt in else_branch {
                    self.compile_stmt(stmt);
                }
                let end = self.here();
                self.chunk.patch_jump(jump_end, end);
            }
            Stmt::While { cond, body, .. } => {
                let loop_start = self.here();
                self.compile_expr(cond);
                let exit = self.chunk.placeholder(Opcode::JumpIfFalse(0), span);
                self.loops.push(LoopCtx::default());
                for stmt in body {
                    self.compile_stmt(stmt);
                }
                let ctx = self.loops.pop().expect("loop context");
                self.emit(Opcode::Jump(loop_start as u32), span);
                let end = self.here();
                self.chunk.patch_jump(exit, end);
                for jump in ctx.break_jumps {
                    self.chunk.patch_jump(jump, end);
                }
                for jump in ctx.continue_jumps {
                    self.chunk.patch_jump(jump, loop_start);
                }
            }
            Stmt::For {
                var, iter, body, ..
            } => self.compile_for(var, iter, body, span),
            Stmt::Return { value, .. } => {
                match value {
                    Some(expr) => self.compile_expr(expr),
                    None => {
                        self.emit(Opcode::Nil, span);
                    }
                }
                self.emit(Opcode::Return, span);
            }
            Stmt::Break { .. } => {
                let jump = self.chunk.placeholder(Opcode::Jump(0), span);
                if let Some(ctx) = self.loops.last_mut() {
                    ctx.break_jumps.push(jump);
                } else {
                    self.diagnostics.push(
                        Diagnostic::error("`break` outside a loop")
                            .with_code("E3001")
                            .with_span(span),
                    );
                }
            }
            Stmt::Continue { .. } => {
                let jump = self.chunk.placeholder(Opcode::Jump(0), span);
                if let Some(ctx) = self.loops.last_mut() {
                    ctx.continue_jumps.push(jump);
                } else {
                    self.diagnostics.push(
                        Diagnostic::error("`continue` outside a loop")
                            .with_code("E3002")
                            .with_span(span),
                    );
                }
            }
        }
    }

    /// Desugars `for x in iter { body }` into a hidden counter loop.
    fn compile_for(&mut self, var: &pebble_ast::Ident, iter: &Expr, body: &[Stmt], span: Span) {
        // Names that cannot collide with user identifiers (they contain a
        // space), used for the hidden iterator and index.
        let iter_name = self.name_const(" #iter");
        let index_name = self.name_const(" #idx");
        let zero = self.intern(Constant::Int(0));
        let one = self.intern(Constant::Int(1));
        let len_name = self.name_const("len");

        self.compile_expr(iter);
        self.emit(Opcode::DefineName(iter_name), span);
        self.emit(Opcode::Const(zero), span);
        self.emit(Opcode::DefineName(index_name), span);

        let loop_start = self.here();
        // `#idx < len(#iter)`: callee first, then its argument.
        self.emit(Opcode::GetName(index_name), span);
        self.emit(Opcode::GetName(len_name), span);
        self.emit(Opcode::GetName(iter_name), span);
        self.emit(Opcode::Call(1), span);
        self.emit(Opcode::Less, span);
        let exit = self.chunk.placeholder(Opcode::JumpIfFalse(0), span);

        self.loops.push(LoopCtx::default());
        // `let x = #iter[#idx];`
        self.emit(Opcode::GetName(iter_name), span);
        self.emit(Opcode::GetName(index_name), span);
        self.emit(Opcode::Index, span);
        let var_name = self.name_const(&var.name);
        self.emit(Opcode::DefineName(var_name), span);
        for stmt in body {
            self.compile_stmt(stmt);
        }
        let increment = self.here();
        // `#idx = #idx + 1`
        self.emit(Opcode::GetName(index_name), span);
        self.emit(Opcode::Const(one), span);
        self.emit(Opcode::Add, span);
        self.emit(Opcode::SetName(index_name), span);
        self.emit(Opcode::Pop, span);
        self.emit(Opcode::Jump(loop_start as u32), span);

        let end = self.here();
        self.chunk.patch_jump(exit, end);
        let ctx = self.loops.pop().expect("loop context");
        for jump in ctx.break_jumps {
            self.chunk.patch_jump(jump, end);
        }
        for jump in ctx.continue_jumps {
            self.chunk.patch_jump(jump, increment);
        }
    }

    // --- expressions ------------------------------------------------------

    fn compile_expr(&mut self, expr: &Expr) {
        let span = expr.span();
        match expr {
            Expr::Nil { .. } => {
                self.emit(Opcode::Nil, span);
            }
            Expr::Bool { value, .. } => {
                self.emit(if *value { Opcode::True } else { Opcode::False }, span);
            }
            Expr::Int { value, .. } => {
                let index = self.intern(Constant::Int(*value));
                self.emit(Opcode::Const(index), span);
            }
            Expr::Float { value, .. } => {
                let index = self.intern(Constant::Float(*value));
                self.emit(Opcode::Const(index), span);
            }
            Expr::Str { value, .. } => {
                let index = self.intern(Constant::Str(value.clone()));
                self.emit(Opcode::Const(index), span);
            }
            Expr::Var { name, .. } => {
                let index = self.name_const(&name.name);
                self.emit(Opcode::GetName(index), span);
            }
            Expr::List { items, .. } => {
                for item in items {
                    self.compile_expr(item);
                }
                self.emit(Opcode::BuildList(items.len() as u32), span);
            }
            Expr::Map { entries, .. } => {
                for (key, value) in entries {
                    self.compile_expr(key);
                    self.compile_expr(value);
                }
                self.emit(Opcode::BuildMap(entries.len() as u32), span);
            }
            Expr::Unary { op, operand, .. } => {
                self.compile_expr(operand);
                self.emit(
                    match op {
                        UnaryOp::Neg => Opcode::Neg,
                        UnaryOp::Not => Opcode::Not,
                    },
                    span,
                );
            }
            Expr::Binary { op, lhs, rhs, .. } => {
                self.compile_expr(lhs);
                self.compile_expr(rhs);
                self.emit(arithmetic_opcode(*op), span);
            }
            Expr::Logical { op, lhs, rhs, .. } => {
                self.compile_expr(lhs);
                let keep = match op {
                    LogicalOp::And => self.chunk.placeholder(Opcode::JumpIfFalseKeep(0), span),
                    LogicalOp::Or => self.chunk.placeholder(Opcode::JumpIfTrueKeep(0), span),
                };
                self.compile_expr(rhs);
                let end = self.here();
                self.chunk.patch_jump(keep, end);
            }
            Expr::Assign { target, value, .. } => {
                match target.as_ref() {
                    Expr::Var { name, .. } => {
                        self.compile_expr(value);
                        let index = self.name_const(&name.name);
                        self.emit(Opcode::SetName(index), span);
                    }
                    Expr::Index { object, index, .. } => {
                        self.compile_expr(object);
                        self.compile_expr(index);
                        self.compile_expr(value);
                        self.emit(Opcode::SetIndex, span);
                    }
                    other => {
                        self.diagnostics.push(
                            Diagnostic::error("invalid assignment target")
                                .with_code("E3003")
                                .with_span(other.span()),
                        );
                        // Emit the value anyway so the stack stays balanced.
                        self.compile_expr(value);
                    }
                }
            }
            Expr::Call { callee, args, .. } => {
                self.compile_expr(callee);
                for arg in args {
                    self.compile_expr(arg);
                }
                self.emit(Opcode::Call(args.len() as u32), span);
            }
            Expr::Index { object, index, .. } => {
                self.compile_expr(object);
                self.compile_expr(index);
                self.emit(Opcode::Index, span);
            }
            Expr::Fn { params, body, .. } => {
                let index = self.compile_function(None, params, body);
                self.emit(Opcode::MakeClosure(index), span);
            }
        }
    }

    fn compile_function(
        &mut self,
        name: Option<String>,
        params: &[pebble_ast::Ident],
        body: &[Stmt],
    ) -> u32 {
        let mut sub = Compiler::new();
        sub.compile_body(body, true);
        self.diagnostics.extend(sub.diagnostics);
        let proto = FunctionProto {
            name,
            params: params.iter().map(|param| param.name.clone()).collect(),
            chunk: sub.chunk,
        };
        self.chunk.add_constant(Constant::Function(Rc::new(proto)))
    }
}

const fn arithmetic_opcode(op: BinaryOp) -> Opcode {
    match op {
        BinaryOp::Add => Opcode::Add,
        BinaryOp::Sub => Opcode::Sub,
        BinaryOp::Mul => Opcode::Mul,
        BinaryOp::Div => Opcode::Div,
        BinaryOp::Rem => Opcode::Rem,
        BinaryOp::Eq => Opcode::Equal,
        BinaryOp::NotEq => Opcode::NotEqual,
        BinaryOp::Lt => Opcode::Less,
        BinaryOp::LtEq => Opcode::LessEqual,
        BinaryOp::Gt => Opcode::Greater,
        BinaryOp::GtEq => Opcode::GreaterEqual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pebble_parser::parse;
    use pebble_span::SourceId;

    fn compile_src(src: &str) -> Compiled {
        let result = parse(SourceId(0), src);
        assert!(result.is_ok(), "parse errors: {:?}", result.diagnostics);
        let compiled = compile(&result.program);
        assert!(
            compiled.is_ok(),
            "compile errors: {:?}",
            compiled.diagnostics
        );
        compiled.compiled
    }

    #[test]
    fn compiles_arithmetic() {
        let compiled = compile_src("1 + 2 * 3;");
        let listing = compiled.chunk().disassemble();
        assert!(listing.contains("Const"), "{listing}");
        assert!(listing.contains("Mul"), "{listing}");
        assert!(listing.contains("Add"), "{listing}");
    }

    #[test]
    fn non_trailing_expression_statements_are_popped() {
        let compiled = compile_src("1 + 2; 3;");
        let listing = compiled.chunk().disassemble();
        assert!(listing.contains("Pop"), "{listing}");
    }

    #[test]
    fn trailing_expression_is_the_program_value() {
        let compiled = compile_src("1; 2;");
        // The last expression is not popped, so the VM can return it.
        let code = compiled.chunk().code();
        assert_eq!(code.last(), Some(&Opcode::Const(1)));
    }

    #[test]
    fn if_compiles_to_a_patched_forward_jump() {
        let compiled = compile_src("if true { 1; } else { 2; }");
        let listing = compiled.chunk().disassemble();
        assert!(listing.contains("JumpIfFalse ->"), "{listing}");
        assert!(listing.contains("Jump ->"), "{listing}");
    }

    #[test]
    fn while_back_edge_points_backwards() {
        let compiled = compile_src("let i = 0; while i < 3 { i = i + 1; }");
        let code = compiled.chunk().code();
        let back = code
            .iter()
            .find_map(|op| match op {
                Opcode::Jump(target) => Some(*target as usize),
                _ => None,
            })
            .expect("a backward jump");
        assert!(back < code.len() - 1);
    }

    #[test]
    fn break_and_continue_are_patched() {
        let compiled = compile_src("while true { if true { break; } continue; }");
        let code = compiled.chunk().code();
        // The loop-exit jump lands just past the loop body.
        let exit = code
            .iter()
            .find_map(|op| match op {
                Opcode::JumpIfFalse(target) => Some(*target as usize),
                _ => None,
            })
            .expect("an exit jump");
        assert_eq!(exit, code.len() - 1, "{code:?}");
        // `continue` (and the back edge) point at the top of the loop.
        assert!(
            code.iter().any(|op| matches!(op, Opcode::Jump(0))),
            "{code:?}"
        );
    }

    #[test]
    fn for_loop_desugars_to_indexing() {
        let compiled = compile_src("for x in [1, 2, 3] { x; }");
        let listing = compiled.chunk().disassemble();
        assert!(listing.contains("BuildList"), "{listing}");
        assert!(listing.contains("Index"), "{listing}");
        assert!(listing.contains("Less"), "{listing}");
    }

    #[test]
    fn functions_become_constants_and_closures() {
        let compiled = compile_src("fn add(a, b) { return a + b; } add(1, 2);");
        let listing = compiled.chunk().disassemble();
        assert!(listing.contains("MakeClosure"), "{listing}");
        assert!(listing.contains("-- function add --"), "{listing}");
        assert!(listing.contains("Return"), "{listing}");
    }

    #[test]
    fn closures_capture_implicitly_by_name() {
        let compiled = compile_src("let f = fn(x) { return x; };");
        let listing = compiled.chunk().disassemble();
        assert!(listing.contains("-- function <anonymous> --"), "{listing}");
    }

    #[test]
    fn logical_operators_short_circuit_with_keeping_jumps() {
        let compiled = compile_src("a and b; a or b;");
        let listing = compiled.chunk().disassemble();
        assert!(listing.contains("JumpIfFalseKeep"), "{listing}");
        assert!(listing.contains("JumpIfTrueKeep"), "{listing}");
    }

    #[test]
    fn constants_are_deduplicated() {
        let compiled = compile_src("1 + 1 + 1; \"x\"; \"x\";");
        let ints = compiled
            .chunk()
            .constants()
            .iter()
            .filter(|c| matches!(c, Constant::Int(1)))
            .count();
        let strings = compiled
            .chunk()
            .constants()
            .iter()
            .filter(|c| matches!(c, Constant::Str(s) if s == "x"))
            .count();
        assert_eq!(ints, 1);
        assert_eq!(strings, 1);
    }

    #[test]
    fn break_outside_a_loop_is_a_diagnostic() {
        let result = parse(SourceId(0), "break;");
        let compiled = compile(&result.program);
        assert!(
            compiled
                .diagnostics
                .iter()
                .any(|d| d.code.as_deref() == Some("E3001"))
        );
    }

    #[test]
    fn operand_bytes_accounts_for_operands() {
        assert_eq!(Opcode::Const(0).operand_bytes(), 4);
        assert_eq!(Opcode::Add.operand_bytes(), 0);
        assert_eq!(Opcode::Add.name(), "Add");
    }

    #[test]
    fn chunk_macro_builds_a_chunk() {
        let chunk = chunk![Opcode::Nil, Opcode::Const(0), Opcode::Return];
        assert_eq!(chunk.len(), 3);
        assert_eq!(chunk.code()[0], Opcode::Nil);
        assert_eq!(chunk.code()[1], Opcode::Const(0));
    }
}
