use lab_macros_step::Hello;

#[derive(Hello)]
struct Anything;

fn main() {
    println!("{}", Anything::hello());
}
