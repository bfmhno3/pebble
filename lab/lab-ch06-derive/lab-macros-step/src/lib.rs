use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Hello)]
pub fn derive_hello(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    eprintln!("deriving Hello for {name}");
    quote! {
        impl #name {
            pub fn hello() -> &'static str {
                concat!("hello from ", stringify!(#name))
            }
        }
    }
    .into()
}

#[proc_macro_derive(SpannedStruct)]
pub fn derive_spanned_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    quote! {
        impl ::pebble_span::Spanned for #name {
            fn span(&self) -> ::pebble_span::Span {
                self.span
            }
        }
    }
    .into()
}

#[proc_macro_derive(Show)]
pub fn derive_show(input: TokenStream) -> TokenStream {
    eprintln!("input tokens: {input}");
    TokenStream::new()
}
