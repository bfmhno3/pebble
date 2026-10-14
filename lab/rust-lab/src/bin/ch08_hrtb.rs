#[allow(dead_code)]
struct Span(u32);

type Native = fn(&mut Vec<String>, Span, &[i64]) -> i64;
type Spelled = for<'a, 'b> fn(&'a mut Vec<String>, Span, &'b [i64]) -> i64;

fn count(_: &mut Vec<String>, _: Span, args: &[i64]) -> i64 {
    args.len() as i64
}

fn main() {
    let a: Native = count;
    let b: Spelled = a;
    let mut log = Vec::new();
    println!("{}", b(&mut log, Span(0), &[1, 2, 3]));
}
