use anyhow::{Context, Result};
use std::fs;

fn load(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("cannot read {path}"))
}

fn main() {
    let error = load("nope.pebble").unwrap_err();
    println!("{{error}}   : {error}");
    println!("{{error:#}}  : {error:#}");
    println!("{{error:?}}  :\n{error:?}");
}
