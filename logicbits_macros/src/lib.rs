mod kitchen_nightmare_impl;

use proc_macro::TokenStream;

use crate::kitchen_nightmare_impl::derive_yuck_tokens;

#[proc_macro_derive(KitchenNightmares, attributes(yuck))]
pub fn derive_kitchen_nightmares(input: TokenStream) -> TokenStream {
    derive_yuck_tokens(input)
}
