fn double(x: i64) -> i64 {
    x * 2
}

fn main() {
    let table: [(&str, fn(i64) -> i64); 2] = [("double", double), ("negate", |x| -x)];
    for (name, f) in table {
        println!("{name}(21) = {}", f(21));
    }
    println!("size_of fn pointer: {}", std::mem::size_of::<fn(i64) -> i64>());
    let offset = 10;
    let add_offset = move |x: i64| x + offset;
    println!("size_of capturing closure: {}", std::mem::size_of_val(&add_offset));
    println!("add_offset(1) = {}", add_offset(1));
}
