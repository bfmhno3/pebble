//! Differential tests: every program here runs on both the tree-walking
//! interpreter and the bytecode VM, and the two rendered results must match.
//!
//! Each function returns a Pebble program; `#[differential]` keeps the
//! function and adds a `#[test]` named `<function>_backends_agree`.

use pebble_macros::differential;

#[differential]
fn closures_keep_private_state() -> &'static str {
    r#"
fn make_counter() {
    let count = 0;
    return fn() {
        count = count + 1;
        return count;
    };
}
let next = make_counter();
[next(), next(), next()];
"#
}

#[differential]
fn recursion_and_arithmetic() -> &'static str {
    r#"
fn fib(n) {
    if n < 2 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}
fib(15);
"#
}

#[differential]
fn loops_with_break_and_continue() -> &'static str {
    r#"
let i = 0;
let total = 0;
while i < 20 {
    i = i + 1;
    if i % 2 == 0 {
        continue;
    }
    if i > 15 {
        break;
    }
    total = total + i;
}
total;
"#
}

#[differential]
fn strings_and_negative_indexing() -> &'static str {
    r#"let s = "pebble"; s[-1] + s[0];"#
}

#[differential]
fn lists_are_shared_by_reference() -> &'static str {
    "let a = []; let b = a; push(b, 1); push(a, 2); a;"
}
