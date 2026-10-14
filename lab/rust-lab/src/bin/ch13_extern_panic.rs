extern "C" fn callback(x: i32) -> i32 {
    if x == 3 {
        panic!("callback exploded");
    }
    x
}

fn main() {
    for i in 1..=5 {
        println!("callback({i}) = {}", callback(i));
    }
}
