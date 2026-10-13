#![forbid(unsafe_code)]
//! The Pebble abstract syntax tree.
//!
//! The AST is where the language stops being text and starts being *structure*.
//! Three Rust ideas do most of the work here:
//!
//! * **Algebraic data types.** [`Expr`] and [`Stmt`] are recursive enums; a
//!   program is a `Vec<Stmt>` and an expression is a tree. Rust's `enum` is the
//!   single feature that makes writing a compiler pleasant.
//! * **`Box`.** Recursive enums need indirection, so child expressions are
//!   `Box<Expr>`.
//! * **Traits + a visitor.** [`Visit`] is a trait with default methods, and
//!   [`AstPrinter`] is one implementation. Deriving [`Spanned`] comes from the
//!   `pebble-macros` crate.

use core::fmt;
use std::string::String;
use std::vec::Vec;

pub use pebble_span::{Span, Spanned};

/// Parses `#[derive(Spanned)]` into scope as a *macro*; the trait of the same
/// name comes from `pebble-span`. The two live in different namespaces, which
/// is why `Spanned` can mean both.
pub use pebble_macros::Spanned;

/// A whole parsed program.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    /// Top-level statements, in source order.
    pub stmts: Vec<Stmt>,
}

impl Program {
    /// Wraps a statement list.
    #[must_use]
    pub fn new(stmts: Vec<Stmt>) -> Self {
        Self { stmts }
    }
}

impl<'a> IntoIterator for &'a Program {
    type Item = &'a Stmt;
    type IntoIter = std::slice::Iter<'a, Stmt>;

    fn into_iter(self) -> Self::IntoIter {
        self.stmts.iter()
    }
}

/// An identifier: its name plus where it appeared.
#[derive(Clone, Debug, PartialEq, Spanned)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    /// Builds an identifier.
    #[must_use]
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// A statement: something that happens, and may change state.
#[derive(Clone, Debug, PartialEq, Spanned)]
pub enum Stmt {
    /// `let name = value;`
    Let {
        name: Ident,
        value: Expr,
        span: Span,
    },
    /// `fn name(params) { body }`
    Fn {
        name: Ident,
        params: Vec<Ident>,
        body: Vec<Stmt>,
        span: Span,
    },
    /// An expression evaluated for its side effects.
    Expr { expr: Expr, span: Span },
    /// `if cond { .. } else { .. }`
    If {
        cond: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Vec<Stmt>,
        span: Span,
    },
    /// `while cond { .. }`
    While {
        cond: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    /// `for var in iter { .. }`
    For {
        var: Ident,
        iter: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    /// `return value?;`
    Return { value: Option<Expr>, span: Span },
    /// `break;`
    Break { span: Span },
    /// `continue;`
    Continue { span: Span },
}

/// An expression: something that produces a value.
#[derive(Clone, Debug, PartialEq, Spanned)]
pub enum Expr {
    /// `nil`
    Nil { span: Span },
    /// `true` / `false`
    Bool { value: bool, span: Span },
    /// `42`
    Int { value: i64, span: Span },
    /// `4.2`
    Float { value: f64, span: Span },
    /// `"hi"`
    Str { value: String, span: Span },
    /// A variable reference.
    Var { name: Ident, span: Span },
    /// `[a, b, c]`
    List { items: Vec<Expr>, span: Span },
    /// `#{ key: value, .. }`
    Map {
        entries: Vec<(Expr, Expr)>,
        span: Span,
    },
    /// `-x`, `not x`
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// `a + b`, `a < b`
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// `a and b`, short-circuiting
    Logical {
        op: LogicalOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// `target = value`
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    /// `callee(args)`
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// `object[index]`
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// An anonymous function / closure.
    Fn {
        params: Vec<Ident>,
        body: Vec<Stmt>,
        span: Span,
    },
}

/// Prefix operators.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-`
    Neg,
    /// `not` / `!`
    Not,
}

/// Infix arithmetic and comparison operators.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
        })
    }
}

/// Short-circuiting logical operators.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

/// A read-only AST visitor with sensible defaults.
///
/// Override only the methods you care about; the rest fall back to a full
/// recursive walk, so a visitor that does nothing visits every node.
pub trait Visit {
    /// Visits a whole program.
    fn visit_program(&mut self, program: &Program) {
        walk_program(self, program);
    }

    /// Visits a statement.
    fn visit_stmt(&mut self, stmt: &Stmt) {
        walk_stmt(self, stmt);
    }

    /// Visits an expression.
    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }
}

/// Recursively visits every statement of a program.
pub fn walk_program<V: Visit + ?Sized>(visitor: &mut V, program: &Program) {
    for stmt in program {
        visitor.visit_stmt(stmt);
    }
}

/// Recursively visits a statement and its children.
pub fn walk_stmt<V: Visit + ?Sized>(visitor: &mut V, stmt: &Stmt) {
    match stmt {
        Stmt::Let { value, .. } => visitor.visit_expr(value),
        Stmt::Fn { body, .. } => {
            for stmt in body {
                visitor.visit_stmt(stmt);
            }
        }
        Stmt::Expr { expr, .. } => visitor.visit_expr(expr),
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            visitor.visit_expr(cond);
            for stmt in then_branch {
                visitor.visit_stmt(stmt);
            }
            for stmt in else_branch {
                visitor.visit_stmt(stmt);
            }
        }
        Stmt::While { cond, body, .. } => {
            visitor.visit_expr(cond);
            for stmt in body {
                visitor.visit_stmt(stmt);
            }
        }
        Stmt::For { iter, body, .. } => {
            visitor.visit_expr(iter);
            for stmt in body {
                visitor.visit_stmt(stmt);
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                visitor.visit_expr(value);
            }
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

/// Recursively visits an expression and its children.
pub fn walk_expr<V: Visit + ?Sized>(visitor: &mut V, expr: &Expr) {
    match expr {
        Expr::Nil { .. }
        | Expr::Bool { .. }
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Str { .. }
        | Expr::Var { .. } => {}
        Expr::List { items, .. } => {
            for item in items {
                visitor.visit_expr(item);
            }
        }
        Expr::Map { entries, .. } => {
            for (key, value) in entries {
                visitor.visit_expr(key);
                visitor.visit_expr(value);
            }
        }
        Expr::Unary { operand, .. } => visitor.visit_expr(operand),
        Expr::Binary { lhs, rhs, .. } | Expr::Logical { lhs, rhs, .. } => {
            visitor.visit_expr(lhs);
            visitor.visit_expr(rhs);
        }
        Expr::Assign { target, value, .. } => {
            visitor.visit_expr(target);
            visitor.visit_expr(value);
        }
        Expr::Call { callee, args, .. } => {
            visitor.visit_expr(callee);
            for arg in args {
                visitor.visit_expr(arg);
            }
        }
        Expr::Index { object, index, .. } => {
            visitor.visit_expr(object);
            visitor.visit_expr(index);
        }
        Expr::Fn { body, .. } => {
            for stmt in body {
                visitor.visit_stmt(stmt);
            }
        }
    }
}

/// Pretty-prints a program as an indented tree.
///
/// This is one concrete [`Visit`] implementation; the interpreter is another.
#[derive(Default)]
pub struct AstPrinter {
    out: String,
    indent: usize,
}

impl AstPrinter {
    /// A fresh printer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes the printer, returning the rendered tree.
    #[must_use]
    pub fn finish(self) -> String {
        self.out
    }

    fn line(&mut self, text: impl fmt::Display) {
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
        self.out.push_str(&text.to_string());
        self.out.push('\n');
    }
}

impl Visit for AstPrinter {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        let label = match stmt {
            Stmt::Let { name, .. } => format!("let {name}"),
            Stmt::Fn { name, params, .. } => {
                format!("fn {name}({})", join_params(params))
            }
            Stmt::Expr { .. } => "expr".to_string(),
            Stmt::If { .. } => "if".to_string(),
            Stmt::While { .. } => "while".to_string(),
            Stmt::For { var, .. } => format!("for {var}"),
            Stmt::Return { .. } => "return".to_string(),
            Stmt::Break { .. } => "break".to_string(),
            Stmt::Continue { .. } => "continue".to_string(),
        };
        self.line(label);
        self.indent += 1;
        walk_stmt(self, stmt);
        self.indent -= 1;
    }

    fn visit_expr(&mut self, expr: &Expr) {
        let label = match expr {
            Expr::Nil { .. } => "nil".to_string(),
            Expr::Bool { value, .. } => format!("bool {value}"),
            Expr::Int { value, .. } => format!("int {value}"),
            Expr::Float { value, .. } => format!("float {value}"),
            Expr::Str { value, .. } => format!("str {value:?}"),
            Expr::Var { name, .. } => format!("var {name}"),
            Expr::List { items, .. } => format!("list[{}]", items.len()),
            Expr::Map { entries, .. } => format!("map{{{} entries}}", entries.len()),
            Expr::Unary { op, .. } => {
                format!("unary {}", if *op == UnaryOp::Neg { "-" } else { "not" })
            }
            Expr::Binary { op, .. } => format!("binary {op}"),
            Expr::Logical { op, .. } => {
                format!(
                    "logical {}",
                    if *op == LogicalOp::And { "and" } else { "or" }
                )
            }
            Expr::Assign { .. } => "assign".to_string(),
            Expr::Call { args, .. } => format!("call({} args)", args.len()),
            Expr::Index { .. } => "index".to_string(),
            Expr::Fn { params, .. } => format!("fn({})", join_params(params)),
        };
        self.line(label);
        self.indent += 1;
        walk_expr(self, expr);
        self.indent -= 1;
    }
}

fn join_params(params: &[Ident]) -> String {
    params
        .iter()
        .map(|param| param.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Convenience wrapper around [`AstPrinter`].
#[must_use]
pub fn pretty(program: &Program) -> String {
    let mut printer = AstPrinter::new();
    printer.visit_program(program);
    printer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy() -> Span {
        Span::DUMMY
    }

    #[test]
    fn derive_spanned_reads_the_span_field() {
        let ident = Ident::new("x", Span::new(pebble_span::SourceId(3), 5, 6));
        assert_eq!(ident.span(), Span::new(pebble_span::SourceId(3), 5, 6));
    }

    #[test]
    fn derive_spanned_works_on_enum_variants() {
        let expr = Expr::Int {
            value: 7,
            span: Span::new(pebble_span::SourceId(0), 10, 11),
        };
        assert_eq!(expr.span(), Span::new(pebble_span::SourceId(0), 10, 11));

        let stmt = Stmt::Break { span: dummy() };
        assert_eq!(stmt.span(), dummy());
    }

    #[test]
    fn printer_walks_the_whole_tree() {
        let program = Program::new(vec![Stmt::Fn {
            name: Ident::new("add", dummy()),
            params: vec![Ident::new("a", dummy()), Ident::new("b", dummy())],
            body: vec![Stmt::Return {
                value: Some(Expr::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::Var {
                        name: Ident::new("a", dummy()),
                        span: dummy(),
                    }),
                    rhs: Box::new(Expr::Var {
                        name: Ident::new("b", dummy()),
                        span: dummy(),
                    }),
                    span: dummy(),
                }),
                span: dummy(),
            }],
            span: dummy(),
        }]);

        let rendered = pretty(&program);
        let expected = "\
fn add(a, b)
  return
    binary +
      var a
      var b
";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn default_visit_reaches_every_expression() {
        struct Counter(usize);
        impl Visit for Counter {
            fn visit_expr(&mut self, expr: &Expr) {
                self.0 += 1;
                walk_expr(self, expr);
            }
        }

        let program = Program::new(vec![Stmt::Let {
            name: Ident::new("x", dummy()),
            value: Expr::List {
                items: vec![
                    Expr::Int {
                        value: 1,
                        span: dummy(),
                    },
                    Expr::Int {
                        value: 2,
                        span: dummy(),
                    },
                ],
                span: dummy(),
            },
            span: dummy(),
        }]);

        let mut counter = Counter(0);
        counter.visit_program(&program);
        // list, 1, 2
        assert_eq!(counter.0, 3);
    }
}
