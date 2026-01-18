// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Security notice for ConfigOpts and FieldOpts:
//!
//! The following fields may contain sensitive data (passwords, tokens, keys):
//! - remote_password
//! - remote_token
//! - remote_client_key
//!
//! WARNING: Hardcoding sensitive values in proc-macro attributes is insecure!
//! Sensitive values will be embedded in the compiled binary and can be extracted
//! using tools like `cargo expand`.
//!
//! RECOMMENDATION: Use environment variables or runtime configuration instead.
//! Example: Instead of `#[config(remote_password = "secret")]`,
//! use `#[config(remote_password_env = "APP_PASSWORD")]` and set the environment variable.
//!

use darling::FromDeriveInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteProtocol {
    Http,
    Etcd,
    Consul,
    #[default]
    Auto,
}

impl std::str::FromStr for RemoteProtocol {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "http" | "https" => Ok(RemoteProtocol::Http),
            "etcd" => Ok(RemoteProtocol::Etcd),
            "consul" => Ok(RemoteProtocol::Consul),
            _ => Ok(RemoteProtocol::Auto),
        }
    }
}

impl quote::ToTokens for RemoteProtocol {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let s = match self {
            RemoteProtocol::Http => "http",
            RemoteProtocol::Etcd => "etcd",
            RemoteProtocol::Consul => "consul",
            RemoteProtocol::Auto => "auto",
        };
        tokens.extend(quote::quote! { #s });
    }
}

impl darling::FromMeta for RemoteProtocol {
    fn from_meta(meta: &syn::Meta) -> Result<Self, darling::Error> {
        match meta {
            syn::Meta::Path(_) => Ok(RemoteProtocol::Auto),
            syn::Meta::List(list) => {
                let s = list.tokens.to_string();
                let s = s.trim_matches('"');
                Ok(s.parse().unwrap_or(RemoteProtocol::Auto))
            }
            syn::Meta::NameValue(name_value) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &name_value.value
                {
                    return Ok(s.value().parse().unwrap_or(RemoteProtocol::Auto));
                }
                Ok(RemoteProtocol::Auto)
            }
        }
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(config))]
pub struct ConfigOpts {
    pub ident: syn::Ident,
    #[darling(default)]
    pub env_prefix: Option<String>,
    #[darling(default)]
    pub strict: Option<bool>,
    #[darling(default)]
    pub watch: Option<bool>,
    #[darling(default)]
    pub format_detection: Option<String>,
    #[darling(default)]
    pub audit_log: Option<bool>,
    #[darling(default)]
    pub audit_log_path: Option<String>,
    #[darling(default)]
    pub validate: Option<ValidateOpt>,
    #[darling(default)]
    pub remote: Option<String>,
    #[darling(default)]
    pub remote_protocol: Option<RemoteProtocol>,
    #[darling(default)]
    pub remote_timeout: Option<String>,
    #[darling(default)]
    pub remote_fallback: Option<bool>,
    #[darling(default)]
    pub remote_username: Option<String>,
    #[darling(default)]
    pub remote_password: Option<String>,
    #[darling(default)]
    pub remote_token: Option<String>,
    #[darling(default)]
    pub remote_ca_cert: Option<String>,
    #[darling(default)]
    pub remote_client_cert: Option<String>,
    #[darling(default)]
    pub remote_client_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidateOpt(pub bool);

impl quote::ToTokens for ValidateOpt {
    fn to_tokens(&self, _tokens: &mut proc_macro2::TokenStream) {}
}

impl darling::FromMeta for ValidateOpt {
    fn from_meta(meta: &syn::Meta) -> Result<Self, darling::Error> {
        match &meta {
            syn::Meta::Path(_) => Ok(ValidateOpt(true)),
            syn::Meta::List(list) => {
                if list.tokens.is_empty() {
                    Ok(ValidateOpt(true))
                } else {
                    let value = list.tokens.clone();
                    if let Ok(ident) = syn::parse2::<syn::Ident>(value) {
                        if ident == "true" {
                            Ok(ValidateOpt(true))
                        } else if ident == "false" {
                            Ok(ValidateOpt(false))
                        } else {
                            Ok(ValidateOpt(true))
                        }
                    } else {
                        Ok(ValidateOpt(true))
                    }
                }
            }
            syn::Meta::NameValue(name_value) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Bool(lit_bool),
                    ..
                }) = &name_value.value
                {
                    Ok(ValidateOpt(lit_bool.value))
                } else if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &name_value.value
                {
                    if lit_str.value() == "true" {
                        Ok(ValidateOpt(true))
                    } else {
                        Ok(ValidateOpt(false))
                    }
                } else {
                    Ok(ValidateOpt(true))
                }
            }
        }
    }
}

impl quote::ToTokens for ConfigOpts {
    fn to_tokens(&self, _tokens: &mut proc_macro2::TokenStream) {}
}

#[derive(Debug, Clone)]
pub struct FieldOpts {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,
    pub attrs: Vec<syn::Attribute>,
    pub description: Option<String>,
    pub default: Option<syn::Expr>,
    pub flatten: bool,
    pub serde_flatten: bool,
    pub skip: bool,
    pub name_config: Option<String>,
    pub name_env: Option<String>,
    #[allow(dead_code)]
    pub name_clap_long: Option<String>,
    #[allow(dead_code)]
    pub name_clap_short: Option<char>,
    #[allow(dead_code)]
    pub validate: Option<String>,
    #[allow(dead_code)]
    pub custom_validate: Option<String>,
    pub sensitive: Option<bool>,
    pub remote: Option<String>,
    pub remote_timeout: Option<String>,
    pub remote_auth: Option<bool>,
    pub remote_username: Option<String>,
    pub remote_password: Option<String>,
    pub remote_token: Option<String>,
    pub remote_tls: Option<bool>,
    pub remote_ca_cert: Option<String>,
    pub remote_client_cert: Option<String>,
    pub remote_client_key: Option<String>,
}

impl FieldOpts {}
