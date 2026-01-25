// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Security warning constants and patterns
const SENSITIVE_FIELD_PATTERNS: &[&str] = &[
    "password",
    "token",
    "secret",
    "key",
    "credential",
    "auth",
    "private",
    "cert",
];

const MAX_INPUT_LENGTH: usize = 10_000;

/// Check if a field name suggests it contains sensitive data
fn is_sensitive_field_name(field_name: &str) -> bool {
    let lower = field_name.to_lowercase();
    SENSITIVE_FIELD_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

/// Check if a value appears to be sensitive (simple heuristic)
fn is_sensitive_value(value: &str) -> bool {
    // Check for common secret patterns
    if value.len() >= 8 {
        // High entropy check: mix of different character types
        let has_uppercase = value.chars().any(|c| c.is_uppercase());
        let has_lowercase = value.chars().any(|c| c.is_lowercase());
        let has_digit = value.chars().any(|c| c.is_ascii_digit());
        let has_special = value.chars().any(|c| !c.is_alphanumeric());

        if has_uppercase && has_lowercase && has_digit && has_special {
            return true;
        }
    }
    false
}

/// Emit a security warning for hardcoded sensitive values
fn emit_security_warning(field_name: &str, _value: &str) {
    // Only log warning, don't fail - maintain backward compatibility
    eprintln!(
        "⚠️  SECURITY WARNING: Hardcoded sensitive value detected for field '{}'.\n\
         Consider using environment variables instead to avoid embedding secrets in compiled code.\n\
         Example: Use #[config({}_env = \"MY_SECRET_VAR\")] instead of #[config({} = \"...\")]",
        field_name,
        field_name.split("_").next().unwrap_or(field_name),
        field_name
    );
}

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as ProcMacro2TokenStream;
use syn::{parse_macro_input, DeriveInput, Meta};

mod codegen;
mod confers_common;
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

/// Properly unescape a Rust string literal content
/// Handles escape sequences: \\\" -> ", \\\\ -> \\\, \n -> newline, \t -> tab
fn unescape_rust_string(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some(&'\\') => {
                    result.push('\\');
                    chars.next();
                }
                Some(&'"') => {
                    result.push('"');
                    chars.next();
                }
                Some(&'n') => {
                    result.push('\n');
                    chars.next();
                }
                Some(&'t') => {
                    result.push('\t');
                    chars.next();
                }
                Some(&'0') => {
                    result.push('\0');
                    chars.next();
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }
    result
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

/// Extract default value with input validation and length limits
fn extract_default_value(tokens_str: &str) -> Option<(String, bool, bool)> {
    // Security: Enforce input length limit to prevent DoS
    if tokens_str.len() > MAX_INPUT_LENGTH {
        eprintln!(
            "⚠️  SECURITY WARNING: Input token length ({}) exceeds maximum allowed ({}). \
             Potential denial of service attack detected.",
            tokens_str.len(),
            MAX_INPUT_LENGTH
        );
        return None;
    }

    if let Some(start) = tokens_str.find("default = ") {
        let after_equals = &tokens_str[start + 10..];

        if after_equals.starts_with('"') {
            let after_first_quote = after_equals
                .get(1..)
                .expect("String should have at least one character after quote");
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
                let inner_value = &after_first_quote[..end];
                // Check if this is already a .to_string() call
                let already_wrapped = inner_value.contains(".to_string()");

                let (value, wrapped) = if already_wrapped {
                    // Extract the string part inside quotes before .to_string()
                    // For input like "\"old_syntax\".to_string()", extract "old_syntax"
                    let before_to_string = inner_value
                        .strip_suffix(".to_string()")
                        .unwrap_or(inner_value);

                    // Manually extract and unescape the string content
                    // before_to_string is something like \"old_syntax\" (with escaped inner quotes)
                    // The format is: \"content\" where \" is an escaped quote
                    // We need to strip the outer \"...\" and unescape inner \"

                    // Check if content is wrapped in escaped quotes: \"...\"
                    let content = if before_to_string.starts_with("\\\"")
                        && before_to_string.ends_with("\\\"")
                    {
                        &before_to_string[2..before_to_string.len() - 2]
                    } else if before_to_string.starts_with('"') && before_to_string.ends_with('"') {
                        // Regular quotes
                        before_to_string
                            .strip_prefix('"')
                            .and_then(|s| s.strip_suffix('"'))
                            .unwrap_or(before_to_string)
                    } else {
                        before_to_string
                    };

                    // Use proper unescape function for any remaining escapes
                    let unescaped = unescape_rust_string(content);

                    (unescaped, true)
                } else {
                    // Parse the inner value as a string literal (simplified syntax)
                    // For input like "./storage", wrapped should be false
                    let parse_str = format!("\"{}\"", inner_value);
                    if let Ok(lit_str) = syn::parse_str::<syn::LitStr>(&parse_str) {
                        (lit_str.value(), false)
                    } else {
                        // Fallback: strip outer quotes and unescape
                        let content = inner_value
                            .strip_prefix('"')
                            .and_then(|s| s.strip_suffix('"'))
                            .unwrap_or(inner_value);
                        let unescaped = unescape_rust_string(content);
                        (unescaped, false)
                    }
                };
                return Some((value, wrapped, true));
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
    let field_name = opts
        .ident
        .as_ref()
        .map(|i| i.to_string())
        .unwrap_or_default();
    let is_sensitive_field = is_sensitive_field_name(&field_name);

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
                let value = s.value();
                // Security check: warn about hardcoded passwords
                if is_sensitive_field || is_sensitive_value(&value) {
                    emit_security_warning("remote_password", &value);
                }
                opts.remote_password = Some(value);
            }
        }
        Some("remote_token") => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                let value = s.value();
                // Security check: warn about hardcoded tokens
                if is_sensitive_field || is_sensitive_value(&value) {
                    emit_security_warning("remote_token", &value);
                }
                opts.remote_token = Some(value);
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
                let value = s.value();
                // Security check: warn about hardcoded private keys
                if is_sensitive_field || is_sensitive_value(&value) {
                    emit_security_warning("remote_client_key", &value);
                }
                opts.remote_client_key = Some(value);
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
                }

                if !is_string {
                    if already_wrapped {
                        let wrapped_str = format!("{}.to_string()", value);
                        if let Ok(expr) = syn::parse_str::<syn::Expr>(&wrapped_str) {
                            opts.default = Some(expr);
                        }
                    }

                    if !already_wrapped {
                        if let Ok(expr) = syn::parse_str::<syn::Expr>(&value) {
                            opts.default = Some(expr);
                        }
                    }
                }
            }

            // Build tokens stream for parsing MetaNameValue
            let tokens_stream: ProcMacro2TokenStream = list.tokens.clone().into_iter().collect();

            // Try to parse as a single MetaNameValue (for simple cases like name_env = "value")
            if let Ok(nv) = syn::parse2::<syn::MetaNameValue>(tokens_stream.clone()) {
                process_meta_name_value(&nv, &mut opts);
            } else {
                // Try to parse individual MetaNameValue pairs
                let mut current_nv_tokens = ProcMacro2TokenStream::new();
                let mut expect_value = false;

                for token in tokens_stream {
                    if let proc_macro2::TokenTree::Ident(ident) = &token {
                        let ident_str = ident.to_string();
                        // Check if this is a keyword that should be treated as a flag
                        if ident_str == "flatten" {
                            opts.flatten = true;
                            continue;
                        } else if ident_str == "skip" {
                            opts.skip = true;
                            continue;
                        }
                    }

                    if let proc_macro2::TokenTree::Punct(punct) = &token {
                        if punct.as_char() == '=' && !expect_value {
                            expect_value = true;
                            continue;
                        }
                    }

                    if expect_value {
                        current_nv_tokens = ProcMacro2TokenStream::new();
                        expect_value = false;
                    }

                    current_nv_tokens.extend(std::iter::once(token));

                    // Try to parse when we have a complete MetaNameValue
                    if current_nv_tokens.clone().into_iter().next().is_some() {
                        // Check if we have an = sign in the stream
                        let has_equals = current_nv_tokens.clone().into_iter().any(|t| {
                            if let proc_macro2::TokenTree::Punct(p) = t {
                                p.as_char() == '='
                            } else {
                                false
                            }
                        });

                        if has_equals {
                            if let Ok(nv) =
                                syn::parse2::<syn::MetaNameValue>(current_nv_tokens.clone())
                            {
                                process_meta_name_value(&nv, &mut opts);
                                current_nv_tokens = ProcMacro2TokenStream::new();
                            }
                        }
                    }
                }

                // Handle remaining tokens in Group
                for item in list.tokens.clone().into_iter() {
                    if let proc_macro2::TokenTree::Group(group) = item {
                        for inner in group.stream().into_iter() {
                            if let Ok(nv) = syn::parse2::<syn::MetaNameValue>(
                                ProcMacro2TokenStream::from(inner),
                            ) {
                                process_meta_name_value(&nv, &mut opts);
                            }
                        }
                    }
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
