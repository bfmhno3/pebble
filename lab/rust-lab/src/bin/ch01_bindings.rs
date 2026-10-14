fn main() {
    let x = 5;
    let x = x * 2; // shadowing: a brand new variable that reuses the name
    {
        let x = "inner"; // shadowing may even change the type
        println!("inner x = {x}");
    }
    println!("outer x = {x}");

    let mut count = 0;
    count += 1;
    count += 1;
    println!("count = {count}");
}
