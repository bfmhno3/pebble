macro_rules! list {
    ($($item:expr),* $(,)?) => {
        vec![$($item),*]
    };
}

fn main() {
    let v = list![1, 2, 3,];
    let w: Vec<i32> = list![];
    println!("{v:?} {w:?}");
}
