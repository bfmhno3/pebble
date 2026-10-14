macro_rules! list {
    ($($item:expr),*) => {
        vec![$($item),*]
    };
}

fn main() {
    let v = list![1, 2, 3,];
    println!("{v:?}");
}
