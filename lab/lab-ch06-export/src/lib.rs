use proc_macro::TokenStream;

/// Not a macro: just a helper.
pub fn helper() -> u32 {
    42
}

#[proc_macro_derive(Nothing)]
pub fn derive_nothing(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
