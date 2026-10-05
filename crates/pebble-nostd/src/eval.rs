//! A stack-machine evaluator for integer arithmetic, built on the real lexer.
//!
//! This is deliberately not a tree-walking interpreter. The token stream is
//! compiled to reverse Polish notation by the shunting-yard algorithm, and a
//! [`Vec`] is then used as the machine stack. That keeps the data model honest:
//! the only heap structure the evaluator needs is a growable array of `i64`,
//! which is precisely what a `no_std` target can still provide through `alloc`.
//!
//! The grammar is a single precedence level deep:
//!
//! ```text
//! expr   := term (('+' | '-') term)*
//! term   := unary (('*' | '/' | '%') unary)*
//! unary  := '-' unary | primary
//! primary := INT | '(' expr ')'
//! ```
//!
//! All arithmetic is checked. Overflowing an `i64` is a program error, not
//! undefined behaviour, so it surfaces as [`EvalError::Overflow`].

use alloc::vec::Vec;

use pebble_lexer::{Token, TokenKind, tokenize};
use pebble_span::SourceMap;

/// Everything that can go wrong while evaluating an expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvalError {
    /// A `/` or `%` had a right-hand side of zero.
    DivisionByZero,
    /// A token appeared where the grammar did not allow it, such as a stray
    /// `)` or two numbers in a row.
    UnexpectedToken,
    /// Input ended before the expression was complete, such as a dangling `+`.
    UnexpectedEnd,
    /// A checked arithmetic operation overflowed `i64`.
    Overflow,
}

/// A binary or unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    /// Unary minus, so `-x` needs no special case in the evaluator.
    Neg,
}

impl Op {
    /// Binding strength; larger binds tighter.
    const fn precedence(self) -> u8 {
        match self {
            Op::Add | Op::Sub => 1,
            Op::Mul | Op::Div | Op::Rem => 2,
            Op::Neg => 3,
        }
    }

    /// Unary minus is right-associative (`- -3` is `-(-3)`); the rest are not.
    const fn is_left_associative(self) -> bool {
        !matches!(self, Op::Neg)
    }
}

/// One instruction of the compiled program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rpn {
    /// Push a literal onto the machine stack.
    Push(i64),
    /// Pop the operand(s) and push the result.
    Apply(Op),
}

/// An entry on the shunting-yard operator stack.
enum StackItem {
    Op(Op),
    /// A `(` marker, popped when its matching `)` is seen.
    LParen,
}

/// Lexes and evaluates `src`, returning its integer value.
///
/// A fresh [`SourceMap`] is created internally, so callers do not have to know
/// about source ids just to evaluate a snippet.
pub fn eval_source(src: &str) -> Result<i64, EvalError> {
    let mut sources = SourceMap::new();
    let id = sources.add("<nostd>", src);
    let (tokens, diagnostics) = tokenize(id, src);
    if !diagnostics.is_empty() {
        // The lexer keeps going after a bad character, so a non-empty
        // diagnostic list means the stream is not the expression we asked for.
        return Err(EvalError::UnexpectedToken);
    }
    eval_rpn(&compile(&tokens)?)
}

/// Shunting-yard: token stream in, reverse Polish notation out.
fn compile(tokens: &[Token<'_>]) -> Result<Vec<Rpn>, EvalError> {
    let mut output: Vec<Rpn> = Vec::new();
    let mut operators: Vec<StackItem> = Vec::new();
    // True while the grammar expects an operand, which is exactly when a `-`
    // is unary rather than subtraction.
    let mut expect_operand = true;

    for token in tokens {
        match &token.kind {
            TokenKind::Int(value) => {
                if !expect_operand {
                    return Err(EvalError::UnexpectedToken);
                }
                output.push(Rpn::Push(*value));
                expect_operand = false;
            }
            TokenKind::LParen => {
                if !expect_operand {
                    return Err(EvalError::UnexpectedToken);
                }
                operators.push(StackItem::LParen);
            }
            TokenKind::RParen => {
                if expect_operand {
                    return Err(EvalError::UnexpectedToken);
                }
                loop {
                    match operators.pop() {
                        Some(StackItem::Op(op)) => output.push(Rpn::Apply(op)),
                        Some(StackItem::LParen) => break,
                        // A `)` with no matching `(`.
                        None => return Err(EvalError::UnexpectedToken),
                    }
                }
                expect_operand = false;
            }
            kind @ (TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent) => {
                let op = match kind {
                    TokenKind::Plus => Op::Add,
                    TokenKind::Minus => Op::Sub,
                    TokenKind::Star => Op::Mul,
                    TokenKind::Slash => Op::Div,
                    TokenKind::Percent => Op::Rem,
                    _ => unreachable!("guarded by the match arm above"),
                };
                if expect_operand {
                    // Only `-` may introduce an operand; `+`/`*`/... cannot.
                    if op == Op::Sub {
                        push_operator(&mut operators, &mut output, Op::Neg);
                    } else {
                        return Err(EvalError::UnexpectedToken);
                    }
                } else {
                    push_operator(&mut operators, &mut output, op);
                    expect_operand = true;
                }
            }
            TokenKind::Eof => break,
            // Identifiers, strings, keywords, and the rest are not part of the
            // arithmetic grammar.
            _ => return Err(EvalError::UnexpectedToken),
        }
    }

    // Flush whatever is left, checking for an unclosed `(` on the way.
    while let Some(item) = operators.pop() {
        match item {
            StackItem::Op(op) => output.push(Rpn::Apply(op)),
            StackItem::LParen => return Err(EvalError::UnexpectedEnd),
        }
    }

    // A trailing binary operator leaves us still wanting an operand.
    if expect_operand && !output.is_empty() {
        return Err(EvalError::UnexpectedEnd);
    }
    Ok(output)
}

/// Pops higher-priority operators, then pushes `op`.
fn push_operator(operators: &mut Vec<StackItem>, output: &mut Vec<Rpn>, op: Op) {
    while let Some(StackItem::Op(top)) = operators.last() {
        let top = *top;
        let should_pop = top.precedence() > op.precedence()
            || (top.precedence() == op.precedence() && op.is_left_associative());
        if !should_pop {
            break;
        }
        output.push(Rpn::Apply(top));
        operators.pop();
    }
    operators.push(StackItem::Op(op));
}

/// Runs the compiled program on a [`Vec`] used as the machine stack.
fn eval_rpn(rpn: &[Rpn]) -> Result<i64, EvalError> {
    let mut stack: Vec<i64> = Vec::new();
    for instruction in rpn {
        match *instruction {
            Rpn::Push(value) => stack.push(value),
            Rpn::Apply(Op::Neg) => {
                let value = stack.pop().ok_or(EvalError::UnexpectedEnd)?;
                stack.push(value.checked_neg().ok_or(EvalError::Overflow)?);
            }
            Rpn::Apply(op) => {
                let rhs = stack.pop().ok_or(EvalError::UnexpectedEnd)?;
                let lhs = stack.pop().ok_or(EvalError::UnexpectedEnd)?;
                let value = match op {
                    Op::Add => lhs.checked_add(rhs),
                    Op::Sub => lhs.checked_sub(rhs),
                    Op::Mul => lhs.checked_mul(rhs),
                    Op::Div => {
                        if rhs == 0 {
                            return Err(EvalError::DivisionByZero);
                        }
                        lhs.checked_div(rhs)
                    }
                    Op::Rem => {
                        if rhs == 0 {
                            return Err(EvalError::DivisionByZero);
                        }
                        lhs.checked_rem(rhs)
                    }
                    Op::Neg => unreachable!("handled by the arm above"),
                };
                stack.push(value.ok_or(EvalError::Overflow)?);
            }
        }
    }

    match stack.len() {
        1 => Ok(stack[0]),
        0 => Err(EvalError::UnexpectedEnd),
        // Leftover operands, e.g. `1 2`.
        _ => Err(EvalError::UnexpectedToken),
    }
}

#[cfg(test)]
mod tests {
    use super::{EvalError, eval_source};

    #[test]
    fn precedence_is_respected() {
        assert_eq!(eval_source("1 + 2 * 3"), Ok(7));
        assert_eq!(eval_source("1 * 2 + 3"), Ok(5));
        assert_eq!(eval_source("10 / 2 - 3"), Ok(2));
        assert_eq!(eval_source("10 % 3"), Ok(1));
    }

    #[test]
    fn parentheses_override_precedence() {
        assert_eq!(eval_source("(1 + 2) * 3"), Ok(9));
        assert_eq!(eval_source("2 * (3 + 4) - 1"), Ok(13));
        assert_eq!(eval_source("((42))"), Ok(42));
    }

    #[test]
    fn unary_minus_works() {
        assert_eq!(eval_source("-3 + 5"), Ok(2));
        assert_eq!(eval_source("2 * -3"), Ok(-6));
        assert_eq!(eval_source("-(2 + 3)"), Ok(-5));
        assert_eq!(eval_source("- -7"), Ok(7));
    }

    #[test]
    fn division_by_zero_is_reported() {
        assert_eq!(eval_source("1 / 0"), Err(EvalError::DivisionByZero));
        assert_eq!(eval_source("1 % 0"), Err(EvalError::DivisionByZero));
    }

    #[test]
    fn overflow_is_reported() {
        assert_eq!(
            eval_source("9223372036854775807 + 1"),
            Err(EvalError::Overflow)
        );
        assert_eq!(
            eval_source("9223372036854775807 * 2"),
            Err(EvalError::Overflow)
        );
        // i64::MIN cannot be negated.
        assert_eq!(
            eval_source("-9223372036854775807 - 1"),
            Ok(i64::MIN),
            "the lower bound itself is representable"
        );
        assert_eq!(
            eval_source("0 - 9223372036854775807 - 2"),
            Err(EvalError::Overflow)
        );
    }

    #[test]
    fn malformed_input_is_reported() {
        assert_eq!(eval_source(""), Err(EvalError::UnexpectedEnd));
        assert_eq!(eval_source("1 +"), Err(EvalError::UnexpectedEnd));
        assert_eq!(eval_source("1 2"), Err(EvalError::UnexpectedToken));
        assert_eq!(eval_source("(1 + 2"), Err(EvalError::UnexpectedEnd));
        assert_eq!(eval_source("1 + 2)"), Err(EvalError::UnexpectedToken));
        assert_eq!(eval_source("* 3"), Err(EvalError::UnexpectedToken));
        assert_eq!(eval_source("let x"), Err(EvalError::UnexpectedToken));
        assert_eq!(eval_source("1 $ 2"), Err(EvalError::UnexpectedToken));
    }
}
