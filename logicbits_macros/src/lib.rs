mod data_objects;
mod derive_kitchen;
mod generate;

use proc_macro::TokenStream;

use crate::derive_kitchen::derive_yuck_tokens;

#[proc_macro_derive(KitchenNightmares, attributes(yuck))]
pub fn derive_kitchen_nightmares(input: TokenStream) -> TokenStream {
    derive_yuck_tokens(input.into()).into()
}
