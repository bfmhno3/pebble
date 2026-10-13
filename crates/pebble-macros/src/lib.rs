//! Procedural macros for the Pebble toolchain.
//!
//! This crate must be a `proc-macro = true` crate: procedural macros run at
//! compile time, in the compiler's process, and speak in tokens. Three flavours
//! live here:
//!
//! * [`macro@Spanned`] - a **derive** macro that writes the boring
//!   `impl pebble_span::Spanned` for AST nodes.
//! * [`opcode_set`] - a **function-like** macro that turns a declarative list of
//!   opcodes into an enum plus a `name()` table.
//! * [`differential`] - an **attribute** macro that turns a function returning a
//!   Pebble program into a test comparing both backends.
//!
//! (A fourth, **declarative** kind, `macro_rules!`, lives next to the code
//! that uses it in `pebble-bytecode`.)

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, ItemEnum, ItemFn, parse_macro_input};

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
                    #(#arms),*
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

/// Generates a bytecode opcode enum, plus metadata, from a plain list.
///
/// ```ignore
/// opcode_set! {
///     pub enum Opcode {
///         Const,
///         Add,
///         Jump(u32),
///     }
/// }
/// ```
///
/// expands to the enum (with `Clone, Copy, PartialEq, Eq, Hash, Debug`) and a
/// `const fn name(self) -> &'static str` used by the disassembler. Writing this
/// by hand is 90% boilerplate and 10% meaning, which is the sweet spot for a
/// function-like macro.
#[proc_macro]
pub fn opcode_set(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemEnum);
    match expand_opcodes(&item) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_opcodes(item: &ItemEnum) -> syn::Result<TokenStream2> {
    let attrs = &item.attrs;
    let vis = &item.vis;
    let name = &item.ident;
    let variants = &item.variants;

    let arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        if variant.fields.is_empty() {
            quote!(#name::#variant_name => stringify!(#variant_name))
        } else {
            quote!(#name::#variant_name(..) => stringify!(#variant_name))
        }
    });

    Ok(quote! {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        #(#attrs)*
        #vis enum #name {
            #variants
        }

        impl #name {
            /// The opcode's name, for disassembly and tracing.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    #(#arms),*
                }
            }
        }
    })
}

/// Turns a function that returns a Pebble program into a test that runs it on both backends and compares the rendered results.
///
/// ```ignore
/// #[differential]
/// fn sums() -> &'static str {
///     "1 + 2;"
/// }
/// ```
///
/// expands to the function itself plus a `#[test] fn sums_backends_agree()`
/// that feeds the program to `pebble_bench::run_interp` and
/// `pebble_bench::run_vm` and asserts the two renderings are equal.
#[proc_macro_attribute]
pub fn differential(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = TokenStream2::from(attr);
    let function = parse_macro_input!(item as ItemFn);
    match expand_differential(&attr, &function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_differential(attr: &TokenStream2, function: &ItemFn) -> syn::Result<TokenStream2> {
    if !attr.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "`#[differential]` takes no arguments",
        ));
    }
    if !function.sig.inputs.is_empty() {
        return Err(syn::Error::new_spanned(
            &function.sig.inputs,
            "a `#[differential]` program takes no parameters",
        ));
    }

    let name = &function.sig.ident;
    let test_name = quote::format_ident!("{}_backends_agree", name);
    let label = name.to_string();

    Ok(quote! {
        #function

        #[test]
        fn #test_name() {
            let src = #name();
            assert_eq!(
                ::pebble_bench::run_interp(src),
                ::pebble_bench::run_vm(src),
                "backends disagree on `{}`",
                #label
            );
        }
    })
}
