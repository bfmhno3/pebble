fn main() {
    let tokens = proc_macro::TokenStream::new();
    println!("{}", tokens.is_empty());
}
