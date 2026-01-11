// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as ProcMacro2TokenStream;
use syn::{parse_macro_input, DeriveInput, Meta};

mod codegen;
mod parse;

fn has_serde_flatten(attrs: &Vec<syn::Attribute>) -> bool {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            if let syn::Meta::List(list) = &attr.meta {
                let s = list.tokens.to_string();
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

fn extract_default_value(tokens_str: &str) -> Option<(String, bool, bool)> {
    if let Some(start) = tokens_str.find("default = ") {
        let after_equals = &tokens_str[start + 10..];

        if after_equals.starts_with('"') {
            let after_first_quote = &after_equals[1..];
            let mut i = 0;
            let mut end_pos = None;

            while i < after_first_quote.len() {
                let c = after_first_quote.chars().nth(i).unwrap();
                if c == '\\' && i + 1 < after_first_quote.len() {
                    let next_char = after_first_quote.chars().nth(i + 1).unwrap();
                    if next_char == '"' {
                        i += 2;
                        continue;
                    }
                }
                if c == '"' {
                    end_pos = Some(i);
                    break;
                }
                i += 1;
            }

            if let Some(end) = end_pos {
                let value_with_quotes = &after_first_quote[..end + 1];
                if let Ok(lit_str) = syn::parse_str::<syn::LitStr>(value_with_quotes) {
                    let value = lit_str.value();
                    let already_wrapped = value.starts_with('"')
                        && value.ends_with('"')
                        && value.contains(".to_string()");
                    return Some((value, already_wrapped, true));
                }
            }
        } else {
            let value = after_equals.trim();
            let already_wrapped = value.contains(".to_string()");
            return Some((value.to_string(), already_wrapped, false));
        }
    }
    None
}

fn process_meta_name_value(nv: &syn::MetaNameValue, opts: &mut parse::FieldOpts) {
    let ident = nv.path.get_ident().map(|i| i.to_string());
    match ident.as_deref() {
        Some("description") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.description = Some(s.value());
            }
        }
        Some("default") => {
            if opts.default.is_none() {
                opts.default = Some(nv.value.clone());
            }
        }
        Some("name") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.name_config = Some(s.value());
            }
        }
        Some("name_env") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.name_env = Some(s.value());
            }
        }
        Some("validate") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.validate = Some(s.value());
            } else if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Bool(b),
                ..
            }) = &nv.value
            {
                if b.value {
                    opts.validate = Some("true".to_string());
                }
            } else {
                let s = quote::quote!(#nv.value).to_string();
                opts.validate = Some(s);
            }
        }
        Some("custom_validate") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.custom_validate = Some(s.value());
            }
        }
        Some("sensitive") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Bool(b),
                ..
            }) = &nv.value
            {
                opts.sensitive = Some(b.value);
            }
        }
        Some("remote") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.remote = Some(s.value());
            }
        }
        Some("remote_timeout") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.remote_timeout = Some(s.value());
            }
        }
        Some("remote_auth") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Bool(b),
                ..
            }) = &nv.value
            {
                opts.remote_auth = Some(b.value);
            }
        }
        Some("remote_username") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.remote_username = Some(s.value());
            }
        }
        Some("remote_password") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.remote_password = Some(s.value());
            }
        }
        Some("remote_token") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.remote_token = Some(s.value());
            }
        }
        Some("remote_tls") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Bool(b),
                ..
            }) = &nv.value
            {
                opts.remote_tls = Some(b.value);
            }
        }
        Some("remote_ca_cert") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.remote_ca_cert = Some(s.value());
            }
        }
        Some("remote_client_cert") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.remote_client_cert = Some(s.value());
            }
        }
        Some("remote_client_key") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                opts.remote_client_key = Some(s.value());
            }
        }
        _ => {}
    }
}

fn parse_field_opts(field: &syn::Field) -> parse::FieldOpts {
    let mut opts = parse::FieldOpts {
        ident: field.ident.clone(),
        ty: field.ty.clone(),
        attrs: field.attrs.clone(),
        description: None,
        default: None,
        flatten: false,
        serde_flatten: false,
        skip: false,
        name_config: None,
        name_env: None,
        name_clap_long: None,
        name_clap_short: None,
        validate: None,
        custom_validate: None,
        sensitive: None,
        remote: None,
        remote_timeout: None,
        remote_auth: None,
        remote_username: None,
        remote_password: None,
        remote_token: None,
        remote_tls: None,
        remote_ca_cert: None,
        remote_client_cert: None,
        remote_client_key: None,
    };

    for attr in &field.attrs {
        if !attr.path().is_ident("config") {
            continue;
        }

        if let Meta::List(list) = &attr.meta {
            let tokens_str = list.tokens.to_string();

            if let Some((value, already_wrapped, is_string)) = extract_default_value(&tokens_str) {
                if is_string {
                    if already_wrapped {
                        let lit = syn::LitStr::new(&value, proc_macro2::Span::call_site());
                        let expr: syn::Expr = syn::parse_quote! { #lit };
                        opts.default = Some(expr);
                    } else {
                        let wrapped_str = format!("\"{}\".to_string()", value);
                        if let Ok(expr) = syn::parse_str::<syn::Expr>(&wrapped_str) {
                            opts.default = Some(expr);
                        }
                    }
                } else {
                    if already_wrapped {
                        let wrapped_str = format!("{}.to_string()", value);
                        if let Ok(expr) = syn::parse_str::<syn::Expr>(&wrapped_str) {
                            opts.default = Some(expr);
                        }
                    } else {
                        if let Ok(expr) = syn::parse_str::<syn::Expr>(&value) {
                            opts.default = Some(expr);
                        }
                    }
                }
            }

            for item in list.tokens.clone().into_iter() {
                match item {
                    proc_macro2::TokenTree::Group(group) => {
                        for inner in group.stream().into_iter() {
                            if let Ok(nv) = syn::parse2::<syn::MetaNameValue>(
                                ProcMacro2TokenStream::from(inner),
                            ) {
                                process_meta_name_value(&nv, &mut opts);
                            }
                        }
                    }
                    proc_macro2::TokenTree::Ident(ident) => {
                        let ident_str = ident.to_string();
                        if ident_str == "flatten" {
                            opts.flatten = true;
                        } else if ident_str == "skip" {
                            opts.skip = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    opts
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
                let mut f = parse_field_opts(field);

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
                .into();
        }
    };

    let has_validate = has_validate_derive(&input);
    codegen::generate_impl(&opts, &fields, has_validate).into()
}
