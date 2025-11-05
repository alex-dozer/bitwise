use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Fields, Ident, LitStr, Type, parse_macro_input, spanned::Spanned,
};

#[derive(Debug)]
enum FieldAttr {
    Diner { pred: String },
    Kitchen { eq: String, pred: String },
    Largeness { pred_prefix: String, heat: Vec<u32> },
    Serve { pred_ns: String },
}

#[proc_macro_derive(KitchenNightmares, attributes(yuck))]
pub fn derive_yuck_facts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // only structs with named fields
    let ty_ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return compile_error(
                    input.span(),
                    "KitchenNightmares only supports structs with named fields",
                );
            }
        },
        _ => return compile_error(input.span(), "KitchenNightmares only supports structs"),
    };

    let kitchen_menu = parse_kitchen_menu(&input.attrs).unwrap_or(1);

    let mod_name = format!("__nightmares_{}", ty_ident.to_string().to_snake_case());
    let mod_ident = format_ident!("{}", mod_name);

    let mut pred_names: Vec<String> = Vec::new();
    let mut extract_stmts: Vec<TokenStream2> = Vec::new();

    for field in fields {
        let fname = field.ident.clone().expect("named fields only");
        let fty = &field.ty;

        for a in parse_field_attrs(&field.attrs) {
            match a {
                FieldAttr::Diner { pred } => {
                    register(&mut pred_names, &pred);
                    let p = rust_ident(&pred);
                    extract_stmts.push(quote! {
                        if self.#fname { s |= #mod_ident::#p.mask(); }
                    });
                }
                FieldAttr::Kitchen { eq, pred } => {
                    register(&mut pred_names, &pred);
                    let p = rust_ident(&pred);
                    let check = match fty {
                        Type::Path(tp) if tp.path.segments.last().unwrap().ident == "String" => {
                            quote! { self.#fname.eq_ignore_ascii_case(#eq) }
                        }
                        _ => quote! { (*self.#fname).eq_ignore_ascii_case(#eq) },
                    };
                    extract_stmts.push(quote! {
                        if #check { s |= #mod_ident::#p.mask();}
                    });
                }
                FieldAttr::Largeness { pred_prefix, heat } => {
                    for t in heat {
                        let name = format!("{}{}", pred_prefix, t);
                        register(&mut pred_names, &name);
                        let p = rust_ident(&name);
                        extract_stmts.push(quote! {
                            if self.#fname > #t { s |= #mod_ident::#p.mask(); }
                        });
                    }
                }
                FieldAttr::Serve { pred_ns } => {
                    register(&mut pred_names, &pred_ns);
                    let p = rust_ident(&pred_ns);
                    extract_stmts.push(quote! {
                        if (500..600).contains(&self.#fname) { s |= #mod_ident::#p.mask(); }
                    });
                }
            }
        }
    }

    if pred_names.is_empty() {
        return compile_error(
            ty_ident.span(),
            "KitchenNightmares: no field predicates detected. Use #[yuck(...)] with heat = \"100,200,500\".",
        );
    }

    let mut variant_idents: Vec<Ident> = Vec::new();
    let mut variant_defs: Vec<TokenStream2> = Vec::new();
    let mut name_to_mask_arms: Vec<TokenStream2> = Vec::new();
    for (i, name) in pred_names.iter().enumerate() {
        let ident = rust_ident(name);
        variant_idents.push(ident.clone());
        let idx = i as u8;
        variant_defs.push(quote! { #ident = #idx });
        let name_str = name.clone();
        name_to_mask_arms.push(quote! { #name_str => Some(#mod_ident::#ident.mask()), });
    }

    let expanded = quote! {
        #[allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
        mod #mod_ident {
            #[repr(u8)]
            pub enum __Pred { #( #variant_defs ),* }
            impl __Pred {
                #[inline] pub const fn mask(self) -> u64 { 1u64 << (self as u8) }
            }
            pub use __Pred::{ #( #variant_idents ),* };
            pub const kitchen_menu: u16 = #kitchen_menu;


        }

        impl #impl_generics #ty_ident #ty_generics #where_clause {
            pub const KITCHEN_MENU: u16 = #kitchen_menu;
            #[inline]
            pub fn pred_mask_by_name(name: &str) -> Option<u64> {
                match name {
                    #( #name_to_mask_arms )*
                    _ => None,
                }
            }
            #[allow(dead_code)]
            #[inline] pub fn facts_mod() -> &'static str { stringify!(#mod_ident) }
        }

        impl #impl_generics ::logicbits::ToBits for #ty_ident #ty_generics #where_clause {
            #[inline]
            fn to_bits(&self) -> u64 {
                let mut s: u64 = 0;
                #( #extract_stmts )*
                s
            }
        }
    };

    expanded.into()
}

fn compile_error(span: Span, msg: &str) -> TokenStream {
    let t = quote::quote_spanned!(span => compile_error!(#msg););
    t.into()
}

fn rust_ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

fn register(acc: &mut Vec<String>, name: &str) {
    if !acc.iter().any(|n| n == name) {
        acc.push(name.to_string());
    }
}

fn parse_kitchen_menu(attrs: &[Attribute]) -> Option<u16> {
    let mut out: Option<u16> = None;
    for a in attrs {
        if a.path().is_ident("yuck") {
            a.parse_nested_meta(|pm| {
                // BEFORE: schema_version
                if pm.path.is_ident("kitchen_menu") {
                    let lit: syn::LitInt = pm.value()?.parse()?;
                    out = lit.base10_parse::<u16>().ok();
                }
                Ok(())
            })
            .ok();
        }
    }
    out
}

fn parse_field_attrs(attrs: &[Attribute]) -> Vec<FieldAttr> {
    let mut out = Vec::new();
    for a in attrs {
        if !a.path().is_ident("yuck") {
            continue;
        }
        let _ = a.parse_nested_meta(|pm| {
            if pm.path.is_ident("kitchen") {
                let mut pred: Option<String> = None;
                pm.parse_nested_meta(|pm2| {
                    if pm2.path.is_ident("pred") {
                        let s: LitStr = pm2.value()?.parse()?;
                        pred = Some(s.value());
                    }
                    Ok(())
                })?;
                if let Some(pred) = pred {
                    out.push(FieldAttr::Diner { pred });
                }
            } else if pm.path.is_ident("diner") {
                let mut pred: Option<String> = None;
                let mut eq: Option<String> = None;
                pm.parse_nested_meta(|pm2| {
                    if pm2.path.is_ident("pred") {
                        let s: LitStr = pm2.value()?.parse()?;
                        pred = Some(s.value());
                    } else if pm2.path.is_ident("eq") {
                        let s: LitStr = pm2.value()?.parse()?;
                        eq = Some(s.value());
                    }
                    Ok(())
                })?;
                if let (Some(pred), Some(eq)) = (pred, eq) {
                    out.push(FieldAttr::Kitchen { eq, pred });
                }
            } else if pm.path.is_ident("serve") {
                let mut pred_ns: Option<String> = None;
                pm.parse_nested_meta(|pm2| {
                    if pm2.path.is_ident("pred_ns") {
                        let s: LitStr = pm2.value()?.parse()?;
                        pred_ns = Some(s.value());
                    }
                    Ok(())
                })?;
                if let Some(pred_ns) = pred_ns {
                    out.push(FieldAttr::Serve { pred_ns });
                }
            } else if pm.path.is_ident("largeness") {
                let mut pred_prefix: Option<String> = None;
                let mut heat: Vec<u32> = Vec::new();
                pm.parse_nested_meta(|pm2| {
                    if pm2.path.is_ident("pred_prefix") {
                        let s: LitStr = pm2.value()?.parse()?;
                        pred_prefix = Some(s.value());
                    } else if pm2.path.is_ident("heat") {
                        // heat MUST be string "100,200,500"
                        let s: LitStr = pm2.value()?.parse()?;
                        for tok in s.value().split(',') {
                            if let Ok(v) = tok.trim().parse::<u32>() {
                                heat.push(v);
                            }
                        }
                    }
                    Ok(())
                })?;
                if let Some(pp) = pred_prefix {
                    out.push(FieldAttr::Largeness {
                        pred_prefix: pp,
                        heat,
                    });
                }
            }
            Ok(())
        });
    }
    out
}
