use crate::generate::{compile_error, generate_for_kitchen};
use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{DeriveInput, parse2};

pub fn derive_yuck_tokens(input: TokenStream2) -> TokenStream2 {
    let ast: DeriveInput = match parse2(input) {
        Ok(x) => x,
        Err(e) => return compile_error(Span::call_site(), &format!("invalid derive input: {e}")),
    };
    generate_for_kitchen(&ast)
}
