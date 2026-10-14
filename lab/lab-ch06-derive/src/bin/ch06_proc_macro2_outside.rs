fn main() {
    let tokens = proc_macro2::TokenStream::new();
    println!("empty = {}", tokens.is_empty());
}
