use std::env;
use std::fs;
use std::process::exit;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "count" {
        eprintln!("usage: {} count <file>", args[0]);
        exit(2);
    }
    let text = fs::read_to_string(&args[2]).unwrap();
    println!("{} bytes, {} lines", text.len(), text.lines().count());
}
