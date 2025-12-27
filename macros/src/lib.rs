// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod codegen;
mod parse;

fn has_serde_flatten(attrs: &Vec<syn::Attribute>) -> bool {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            if let syn::Meta::List(list) = &attr.meta {
                let s = list.tokens.to_string();
                // 在 token 流中简单检查是否包含 "flatten"
                if s.contains("flatten") {
                    return true;
                }
            }
        }
    }
    false
}

fn has_validate_derive(input: &syn::DeriveInput) -> bool {
    for attr in &input.attrs {
        if attr.path().is_ident("derive") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens_str = list.tokens.to_string();
                if tokens_str.contains("Validate") {
                    return true;
                }
            }
        }
    }
    false
}

#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let opts = match parse::ConfigOpts::from_derive_input(&input) {
        Ok(val) => val,
        Err(err) => return err.write_errors().into(),
    };

    let fields = match &input.data {
        syn::Data::Struct(data) => {
            let mut field_opts = Vec::new();
            for field in &data.fields {
                let mut f = match parse::FieldOpts::from_field(field) {
                    Ok(f) => f,
                    Err(e) => return e.write_errors().into(),
                };

                // Check if field has serde(flatten) and set flatten flag
                if has_serde_flatten(&field.attrs) {
                    f.serde_flatten = true;
                    f.flatten = true;
                }
                field_opts.push(f);
            }
            field_opts
        }
        _ => {
            return syn::Error::new_spanned(input, "Config can only be derived for structs")
                .to_compile_error()
                .into()
        }
    };

    let has_validate = has_validate_derive(&input);
    codegen::generate_impl(&opts, &fields, has_validate).into()
}
