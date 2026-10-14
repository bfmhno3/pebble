use proc_macro::TokenStream;

#[proc_macro_derive(Boom)]
pub fn derive_boom(_input: TokenStream) -> TokenStream {
    panic!("this macro does not like its input");
}
