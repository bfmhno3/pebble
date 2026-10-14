use std::collections::HashMap;

fn main() {
    let mut env: HashMap<String, i64> = HashMap::new();
    env.insert("x".to_string(), 1);
    let name: &str = "x";
    println!("{:?}", env.get(name));
    println!("{:?}", env.get(&"y".to_string()));
}
