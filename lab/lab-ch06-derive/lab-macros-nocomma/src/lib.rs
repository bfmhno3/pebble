//! Procedural macros for the Pebble toolchain.
//!
//! This crate must be a `proc-macro = true` crate: procedural macros run at
//! compile time, in the compiler's process, and speak in tokens. So far it has
//! one flavour:
//!
//! * [`macro@Spanned`] - a **derive** macro that writes the boring
//!   `impl pebble_span::Spanned` for AST nodes.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derives [`pebble_span::Spanned`](https://docs.rs/pebble-span).
///
/// Works on structs and enums that carry a field literally named `span`:
///
/// ```ignore
/// #[derive(Spanned)]
/// struct Ident { name: String, span: Span }
///
/// #[derive(Spanned)]
/// enum Expr {
///     Int { value: i64, span: Span },
///     Var { name: Ident, span: Span },
/// }
/// ```
///
/// For an enum, *every* variant must have the `span` field, because the derived
/// `span()` matches on the variant and returns its span.
#[proc_macro_derive(Spanned)]
pub fn derive_spanned(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_spanned(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_spanned(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => {
            let member = span_member(&data.fields)?;
            // `Span` is `Copy`, so reading the field does not move anything.
            quote!(self.#member)
        }
        Data::Enum(data) => {
            let arms = data
                .variants
                .iter()
                .map(|variant| {
                    let variant_name = &variant.ident;
                    let member = span_member(&variant.fields)?;
                    Ok(quote!(Self::#variant_name { #member, .. } => * #member))
                })
                .collect::<syn::Result<Vec<_>>>()?;
            quote! {
                match self {
                    #(#arms)*
                }
            }
        }
        Data::Union(data) => {
            return Err(syn::Error::new_spanned(
                data.union_token,
                "`Spanned` cannot be derived for unions",
            ));
        }
    };

    Ok(quote! {
        impl #impl_generics ::pebble_span::Spanned for #name #ty_generics #where_clause {
            fn span(&self) -> ::pebble_span::Span {
                #body
            }
        }
    })
}

fn span_member(fields: &Fields) -> syn::Result<syn::Ident> {
    match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .find(|field| field.ident.as_ref().is_some_and(|id| id == "span"))
            .and_then(|field| field.ident.clone())
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    fields,
                    "expected a field named `span` for `#[derive(Spanned)]`",
                )
            }),
        _ => Err(syn::Error::new_spanned(
            fields,
            "`#[derive(Spanned)]` requires named fields with a `span` field",
        )),
    }
}
