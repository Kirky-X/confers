// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Security constants for sensitive data detection

/// Check if a value should trigger security warnings
fn should_warn_about_value(value: &str, field_name: &str) -> bool {
    // Check field name patterns
    if is_sensitive_field_name(field_name) {
        return true;
    }

    // Check value entropy (simplified check for high-entropy strings)
    is_sensitive_value(value)
}

use crate::confers_common::is_sensitive_field_name;
use crate::confers_common::is_sensitive_value;
use crate::parse::{ConfigOpts, FieldOpts, RemoteProtocol};
use proc_macro2::TokenStream;
use quote::quote;
use std::str::FromStr;
use syn::{Attribute, Expr, Meta, Type};

fn is_string_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Lit(expr_lit) if matches!(expr_lit.lit, syn::Lit::Str(_)))
}

fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "String";
        }
    }
    false
}

/// 检查字段是否有 serde(flatten) 属性
fn has_serde_flatten(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            if let Meta::List(list) = &attr.meta {
                let s = list.tokens.to_string();
                // 在 token 流中简单检查是否包含 "flatten"
                // 覆盖基本用例如 #[serde(flatten)] 或 #[serde(default, flatten)]
                if s.contains("flatten") {
                    return true;
                }
            }
        }
    }
    false
}

#[allow(dead_code)]
fn get_shadow_type(ty: &Type) -> Option<TokenStream> {
    if let Type::Path(type_path) = ty {
        let mut new_path = type_path.clone();
        if let Some(last_segment) = new_path.path.segments.last_mut() {
            let ident = &last_segment.ident;
            let new_ident = quote::format_ident!("{}ClapShadow", ident);
            last_segment.ident = new_ident;
            return Some(quote! { #new_path });
        }
    }
    None
}

fn is_option_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

/// 检查类型是否为 Clap 可以处理的原始类型
fn is_primitive_type(ty: &Type) -> bool {
    let type_string = quote!(#ty).to_string();
    matches!(
        type_string.as_str(),
        "u8" | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
            | "bool"
            | "String"
            | "char"
    ) || type_string.starts_with("Option <")
        || type_string.starts_with("Vec <")
}

/// 从 URL 或显式设置中检测远程协议
fn detect_protocol(
    url: &Option<String>,
    explicit_protocol: &Option<RemoteProtocol>,
) -> RemoteProtocol {
    if let Some(protocol) = explicit_protocol {
        if *protocol != RemoteProtocol::Auto {
            return *protocol;
        }
    }

    if let Some(url_str) = url {
        let url_lower = url_str.to_lowercase();
        if url_lower.starts_with("http://") || url_lower.starts_with("https://") {
            return RemoteProtocol::Http;
        }
        if url_lower.starts_with("etcd://") || url_lower.starts_with("etcds://") {
            return RemoteProtocol::Etcd;
        }
        if url_lower.starts_with("consul://") {
            return RemoteProtocol::Consul;
        }
    }

    RemoteProtocol::Http // 默认为 http
}

/// 为字段列表生成 Clap 字段，处理 flatten 属性
#[allow(dead_code)]
fn generate_clap_fields(fields: &[FieldOpts]) -> Vec<TokenStream> {
    let mut clap_fields = Vec::new();

    for field in fields {
        if field.skip {
            continue;
        }

        if field.flatten {
            // 使用影子类型处理展平的字段
            let name = &field.ident;
            let ty = &field.ty;

            if let Some(shadow_ty) = get_shadow_type(ty) {
                clap_fields.push(quote! {
                    #[command(flatten)]
                    pub #name: #shadow_ty,
                });
            }
            continue;
        }

        // 处理不是原始类型的嵌套结构体类型
        // 检查是否为自定义结构体类型（不是原始类型）
        let name = &field.ident;
        let ty = &field.ty;

        let is_custom_struct = {
            let type_string = quote!(#ty).to_string();
            !matches!(
                type_string.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "f32"
                    | "f64"
                    | "bool"
                    | "String"
                    | "char"
                    | "Vec < u8 >"
                    | "Vec < u16 >"
                    | "Vec < u32 >"
                    | "Vec < u64 >"
                    | "Vec < u128 >"
                    | "Vec < usize >"
                    | "Vec < i8 >"
                    | "Vec < i16 >"
                    | "Vec < i32 >"
                    | "Vec < i64 >"
                    | "Vec < i128 >"
                    | "Vec < isize >"
                    | "Vec < f32 >"
                    | "Vec < f64 >"
                    | "Vec < bool >"
                    | "Vec < String >"
                    | "Vec < char >"
            ) && !type_string.starts_with("Option <")
        };

        if is_custom_struct {
            // For custom struct types, automatically flatten them
            if let Some(shadow_ty) = get_shadow_type(ty) {
                clap_fields.push(quote! {
                    #[command(flatten)]
                    pub #name: #shadow_ty,
                });
            }
            continue;
        }

        let name = &field.ident;
        let ty = &field.ty;
        let description = &field.description;
        let long_name = &field.name_clap_long;
        let short_name = &field.name_clap_short;

        // 检查是否为 Option<T>
        let inner_ty = is_option_type(ty).unwrap_or(ty);

        // Only generate clap fields for primitive types that can be parsed from strings
        let type_string = quote!(#inner_ty).to_string();
        let is_primitive = matches!(
            type_string.as_str(),
            "u8" | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "f32"
                | "f64"
                | "bool"
                | "String"
                | "char"
        );

        if !is_primitive {
            continue;
        }

        let long_attr = if let Some(long) = long_name {
            quote! { long = #long }
        } else {
            let field_name = name.as_ref().unwrap().to_string();
            quote! { long = #field_name }
        };

        let short_attr = if let Some(short) = short_name {
            quote! { short = #short }
        } else {
            quote! {}
        };

        let help_attr = if let Some(desc) = description {
            quote! { help = #desc }
        } else {
            quote! {}
        };

        // 构建参数属性，过滤掉空的属性
        let mut arg_attrs = Vec::new();
        arg_attrs.push(long_attr);
        if !short_attr.is_empty() {
            arg_attrs.push(short_attr);
        }
        if !help_attr.is_empty() {
            arg_attrs.push(help_attr);
        }

        // 在 ClapShadow 中使用 Option<T> 以允许可选参数
        // 如果原始类型已经是 Option<T>，直接使用它
        // 如果原始类型是 T，将其包装在 Option<T> 中用于 CLI 参数（因为 CLI 参数通常是可选的覆盖）
        let clap_ty = if is_option_type(ty).is_some() {
            quote! { #ty }
        } else {
            quote! { Option<#ty> }
        };

        clap_fields.push(quote! {
            #[arg(#(#arg_attrs),*)]
            pub #name: #clap_ty,
        });
    }

    clap_fields
}

fn get_custom_validator(field: &FieldOpts) -> Option<String> {
    if let Some(v) = &field.custom_validate {
        return Some(v.clone());
    }

    // 在转发的验证属性中搜索
    for attr in &field.attrs {
        if attr.path().is_ident("validate") {
            if let Meta::List(list) = &attr.meta {
                // 尝试找到 custom = "..."
                let s = list.tokens.to_string();
                if let Some(start) = s.find("custom =") {
                    let rest = &s[start + 8..];
                    if let Some(quote_start) = rest.find('"') {
                        let after_quote = &rest[quote_start + 1..];
                        if let Some(quote_end) = after_quote.find('"') {
                            return Some(after_quote[..quote_end].to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_min_max<T>(inner: &str, min_key: &str, max_key: &str) -> (Option<T>, Option<T>)
where
    T: FromStr + Copy,
{
    let mut min_val: Option<T> = None;
    let mut max_val: Option<T> = None;

    if let Some(min_start) = inner.find(min_key) {
        let min_part = &inner[min_start + min_key.len()..];
        if let Some(min_end) = min_part.find([',', ')']) {
            if let Ok(val) = min_part[..min_end].trim().parse::<T>() {
                min_val = Some(val);
            }
        } else if let Ok(val) = min_part.trim().parse::<T>() {
            min_val = Some(val);
        }
    }

    if let Some(max_start) = inner.find(max_key) {
        let max_part = &inner[max_start + max_key.len()..];
        if let Some(max_end) = max_part.find([',', ')']) {
            if let Ok(val) = max_part[..max_end].trim().parse::<T>() {
                max_val = Some(val);
            }
        } else if let Ok(val) = max_part.trim().parse::<T>() {
            max_val = Some(val);
        }
    }

    (min_val, max_val)
}

fn parse_range_validation(validate_str: &str) -> Option<(i64, i64)> {
    if validate_str.starts_with("range(") && validate_str.ends_with(')') {
        let inner = &validate_str[6..validate_str.len() - 1];
        let (min_val, max_val) = extract_min_max::<i64>(inner, "min =", "max =");

        if min_val.is_some() || max_val.is_some() {
            let min = min_val.unwrap_or(i64::MIN);
            let max = max_val.unwrap_or(i64::MAX);
            return Some((min, max));
        }
    }
    None
}

fn parse_length_validation(validate_str: &str) -> Option<(u64, u64)> {
    if validate_str.starts_with("length(") && validate_str.ends_with(')') {
        let inner = &validate_str[7..validate_str.len() - 1];
        let (min_val, max_val) = extract_min_max::<u64>(inner, "min =", "max =");

        if min_val.is_some() || max_val.is_some() {
            let min = min_val.unwrap_or(0);
            let max = max_val.unwrap_or(u64::MAX);
            return Some((min, max));
        }
    }
    None
}

pub fn generate_impl(
    opts: &ConfigOpts,
    fields: &[FieldOpts],
    has_validate_derive: bool,
) -> TokenStream {
    let struct_name = &opts.ident;
    let env_prefix = opts.env_prefix.as_deref().unwrap_or("");
    let app_name = opts.app_name.clone();
    let strict = opts
        .strict
        .or_else(|| opts.validate.as_ref().map(|v| v.0))
        .unwrap_or(false);

    // Conditional code generation for features

    let apply_app_name = if let Some(name) = &app_name {
        quote! { loader = loader.with_app_name(#name); }
    } else {
        quote! {}
    };

    let apply_format_detection = if let Some(val) = &opts.format_detection {
        quote! { loader = loader.with_format_detection(#val); }
    } else {
        quote! {}
    };

    let apply_remote = if let Some(val) = &opts.remote {
        quote! {
            #[cfg(feature = "remote")]
            { loader = loader.with_remote_config(#val); }
        }
    } else {
        quote! {}
    };

    let apply_remote_timeout = if let Some(val) = &opts.remote_timeout {
        quote! {
            #[cfg(feature = "remote")]
            { loader = loader.with_remote_timeout(#val); }
        }
    } else {
        quote! {}
    };

    let apply_remote_fallback = if let Some(val) = opts.remote_fallback {
        quote! {
            #[cfg(feature = "remote")]
            { loader = loader.with_remote_fallback(#val); }
        }
    } else {
        quote! {}
    };

    let apply_remote_auth = {
        let mut tokens = TokenStream::new();
        if let Some(val) = &opts.remote_username {
            tokens.extend(quote! {
                #[cfg(feature = "remote")]
                { loader = loader.with_remote_username(#val); }
            });
        }
        if let Some(val) = &opts.remote_password {
            // Security: Add warning about hardcoded passwords in generated code
            if should_warn_about_value(val, "password") {
                tokens.extend(quote! {
                    #[cfg(feature = "remote")]
                    {
                        #[cfg(debug_assertions)]
                        eprintln!("⚠️  SECURITY WARNING: Hardcoded password detected in generated code. \
                                 This password may be visible in compiled binaries and through `cargo expand`. \
                                 Consider using environment variables instead.");
                        loader = loader.with_remote_password(#val);
                    }
                });
            } else {
                tokens.extend(quote! {
                    #[cfg(feature = "remote")]
                    { loader = loader.with_remote_password(#val); }
                });
            }
        }
        if let Some(val) = &opts.remote_token {
            // Security: Add warning about hardcoded tokens in generated code
            if should_warn_about_value(val, "token") {
                tokens.extend(quote! {
                    #[cfg(feature = "remote")]
                    {
                        #[cfg(debug_assertions)]
                        eprintln!("⚠️  SECURITY WARNING: Hardcoded token detected in generated code. \
                                 This token may be visible in compiled binaries and through `cargo expand`. \
                                 Consider using environment variables instead.");
                        loader = loader.with_remote_token(#val);
                    }
                });
            } else {
                tokens.extend(quote! {
                    #[cfg(feature = "remote")]
                    { loader = loader.with_remote_token(#val); }
                });
            }
        }
        if tokens.is_empty() {
            quote! {}
        } else {
            tokens
        }
    };

    let apply_remote_tls = {
        let mut tokens = TokenStream::new();
        if let Some(val) = &opts.remote_ca_cert {
            tokens.extend(quote! {
                #[cfg(feature = "remote")]
                { loader = loader.with_remote_ca_cert(#val); }
            });
        }
        if let Some(val) = &opts.remote_client_cert {
            tokens.extend(quote! {
                #[cfg(feature = "remote")]
                { loader = loader.with_remote_client_cert(#val); }
            });
        }
        if let Some(val) = &opts.remote_client_key {
            // Security: Add warning about hardcoded private keys in generated code
            if should_warn_about_value(val, "private_key") {
                tokens.extend(quote! {
                    #[cfg(feature = "remote")]
                    {
                        #[cfg(debug_assertions)]
                        eprintln!("⚠️  SECURITY WARNING: Hardcoded private key detected in generated code. \
                                 Private keys should never be embedded in compiled binaries. \
                                 This is a critical security vulnerability.");
                        loader = loader.with_remote_client_key(#val);
                    }
                });
            } else {
                tokens.extend(quote! {
                    #[cfg(feature = "remote")]
                    { loader = loader.with_remote_client_key(#val); }
                });
            }
        }
        if tokens.is_empty() {
            quote! {}
        } else {
            tokens
        }
    };

    // Generate field-level remote configuration with protocol-specific attribute injection
    let field_remote_configs: Vec<TokenStream> = fields
        .iter()
        .filter(|f| {
            f.remote.is_some()
                || f.remote_timeout.is_some()
                || f.remote_auth.is_some()
                || f.remote_tls.is_some()
                || f.remote_ca_cert.is_some()
                || f.remote_client_cert.is_some()
                || f.remote_client_key.is_some()
        })
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();

            let remote_url = &f.remote;
            let remote_timeout = &f.remote_timeout;
            let remote_auth = f.remote_auth;
            let remote_username = &f.remote_username;
            let remote_password = &f.remote_password;
            let remote_token = &f.remote_token;
            let remote_tls = f.remote_tls;
            let remote_ca_cert = &f.remote_ca_cert;
            let remote_client_cert = &f.remote_client_cert;
            let remote_client_key = &f.remote_client_key;

    // 检测此字段的协议
            let protocol = detect_protocol(remote_url, &opts.remote_protocol);

            let mut config_tokens = TokenStream::new();

            // Common field name setting
            if remote_url.is_some() {
                config_tokens.extend(quote! {
                    #[cfg(feature = "remote")]
                    {
                        loader = loader.with_field_name(stringify!(#field_name).to_string());
                    }
                });
            }

            // 协议特定配置
            match protocol {
                RemoteProtocol::Http => {
                    if let Some(url) = remote_url {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_config(#url); }
                        });
                    }

                    if let Some(timeout) = remote_timeout {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_timeout(#timeout); }
                        });
                    }

                    if remote_auth == Some(true) {
                        if let (Some(username), Some(password)) = (remote_username, remote_password) {
                            config_tokens.extend(quote! {
                                #[cfg(feature = "remote")]
                                { loader = loader.with_remote_auth(#username, #password); }
                            });
                        }
                    }

                    if remote_tls == Some(true) {
                        let ca_cert = remote_ca_cert
                            .as_ref()
                            .map(|c| quote! { #c })
                            .unwrap_or(quote! { "" });
                        let client_cert = remote_client_cert
                            .as_ref()
                            .map(|c| quote! { Some(#c.to_string()) })
                            .unwrap_or(quote! { None });
                        let client_key = remote_client_key
                            .as_ref()
                            .map(|c| quote! { Some(#c.to_string()) })
                            .unwrap_or(quote! { None });

                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_tls(#ca_cert, #client_cert, #client_key); }
                        });
                    } else if let Some(ca_cert) = remote_ca_cert {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_ca_cert(#ca_cert); }
                        });
                    }
                }

                RemoteProtocol::Etcd => {
                    if let Some(url) = remote_url {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_config(#url); }
                        });
                    }

                    if let Some(timeout) = remote_timeout {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_timeout(#timeout); }
                        });
                    }

                    if remote_tls == Some(true) {
                        let ca_cert = remote_ca_cert
                            .as_ref()
                            .map(|c| quote! { #c })
                            .unwrap_or(quote! { "" });
                        let client_cert = remote_client_cert
                            .as_ref()
                            .map(|c| quote! { Some(#c.to_string()) })
                            .unwrap_or(quote! { None });
                        let client_key = remote_client_key
                            .as_ref()
                            .map(|c| quote! { Some(#c.to_string()) })
                            .unwrap_or(quote! { None });

                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_tls(#ca_cert, #client_cert, #client_key); }
                        });
                    } else if let Some(ca_cert) = remote_ca_cert {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_ca_cert(#ca_cert); }
                        });
                    }
                }

                RemoteProtocol::Consul => {
                    if let Some(url) = remote_url {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_config(#url); }
                        });
                    }

                    if let Some(timeout) = remote_timeout {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_timeout(#timeout); }
                        });
                    }

                    if let Some(token) = remote_token {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_token(#token); }
                        });
                    }

                    if remote_tls == Some(true) {
                        let ca_cert = remote_ca_cert
                            .as_ref()
                            .map(|c| quote! { #c })
                            .unwrap_or(quote! { "" });
                        let client_cert = remote_client_cert
                            .as_ref()
                            .map(|c| quote! { Some(#c.to_string()) })
                            .unwrap_or(quote! { None });
                        let client_key = remote_client_key
                            .as_ref()
                            .map(|c| quote! { Some(#c.to_string()) })
                            .unwrap_or(quote! { None });

                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_tls(#ca_cert, #client_cert, #client_key); }
                        });
                    } else if let Some(ca_cert) = remote_ca_cert {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_ca_cert(#ca_cert); }
                        });
                    }
                }

                RemoteProtocol::Auto => {
                    if let Some(url) = remote_url {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_config(#url); }
                        });
                    }

                    if let Some(timeout) = remote_timeout {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_timeout(#timeout); }
                        });
                    }

                    if remote_auth == Some(true) {
                        if let (Some(username), Some(password)) = (remote_username, remote_password) {
                            config_tokens.extend(quote! {
                                #[cfg(feature = "remote")]
                                { loader = loader.with_remote_auth(#username, #password); }
                            });
                        }
                    }

                    if let Some(token) = remote_token {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_token(#token); }
                        });
                    }

                    if remote_tls == Some(true) {
                        let ca_cert = remote_ca_cert
                            .as_ref()
                            .map(|c| quote! { #c })
                            .unwrap_or(quote! { "" });
                        let client_cert = remote_client_cert
                            .as_ref()
                            .map(|c| quote! { Some(#c.to_string()) })
                            .unwrap_or(quote! { None });
                        let client_key = remote_client_key
                            .as_ref()
                            .map(|c| quote! { Some(#c.to_string()) })
                            .unwrap_or(quote! { None });

                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_tls(#ca_cert, #client_cert, #client_key); }
                        });
                    } else if let Some(ca_cert) = remote_ca_cert {
                        config_tokens.extend(quote! {
                            #[cfg(feature = "remote")]
                            { loader = loader.with_remote_ca_cert(#ca_cert); }
                        });
                    }
                }
            }

            config_tokens
        })
        .collect();

    let apply_audit = {
        let mut tokens = TokenStream::new();
        if let Some(val) = opts.audit_log {
            tokens.extend(quote! {
                #[cfg(feature = "audit")]
                { loader = loader.with_audit_log(#val); }
            });
        }
        if let Some(val) = &opts.audit_log_path {
            tokens.extend(quote! {
                #[cfg(feature = "audit")]
                { loader = loader.with_audit_log_path(#val); }
            });
        }
        if tokens.is_empty() {
            quote! {}
        } else {
            tokens
        }
    };

    let apply_watch = if let Some(watch) = opts.watch {
        if watch {
            quote! {
                #[cfg(feature = "watch")]
                { loader = loader.with_watch(true); }
            }
        } else {
            quote! {
                #[cfg(feature = "watch")]
                { loader = loader.with_watch(false); }
            }
        }
    } else {
        quote! {}
    };

    // 为每个字段生成 Clap 参数
    #[cfg(feature = "cli")]
    let clap_fields = generate_clap_fields(fields);
    #[cfg(not(feature = "cli"))]
    let clap_fields: Vec<proc_macro2::TokenStream> = Vec::new();

    // 生成字段名称字符串映射（仅包含原始类型、非展平、非跳过的字段）
    let _field_names: Vec<_> = fields
        .iter()
        .filter(|f| {
            if f.flatten || f.skip {
                return false;
            }
            // 仅包含可由 Clap 处理的原始类型
            let ty = &f.ty;
            let type_string = quote!(#ty).to_string();
            let is_primitive = matches!(
                type_string.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "f32"
                    | "f64"
                    | "bool"
                    | "String"
                    | "char"
            ) || type_string.starts_with("Option <")
                || type_string.starts_with("Vec <");
            is_primitive
        })
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            name.to_string()
        })
        .collect();

    // Generate field access for mapping (only primitive, non-flattened, non-skipped fields)
    let _field_access: Vec<_> = fields
        .iter()
        .filter(|f| {
            if f.flatten || f.skip {
                return false;
            }
            // Only include primitive types that can be handled by Clap
            let ty = &f.ty;
            let type_string = quote!(#ty).to_string();
            let is_primitive = matches!(
                type_string.as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "f32"
                    | "f64"
                    | "bool"
                    | "String"
                    | "char"
            ) || type_string.starts_with("Option <")
                || type_string.starts_with("Vec <");
            is_primitive
        })
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            quote! { self.#name }
        })
        .collect();

    // 分离 Option 和非 Option 字段用于 to_cli_args
    let option_field_names: Vec<_> = fields
        .iter()
        .filter(|f| {
            if f.flatten || f.skip {
                return false;
            }
            // Only include primitive types that can be handled by Clap
            if let Some(inner_ty) = is_option_type(&f.ty) {
                is_primitive_type(inner_ty)
            } else {
                false
            }
        })
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            name.to_string()
        })
        .collect();

    let option_field_access: Vec<_> = fields
        .iter()
        .filter(|f| {
            if f.flatten || f.skip {
                return false;
            }
            // Only include primitive types that can be handled by Clap
            if let Some(inner_ty) = is_option_type(&f.ty) {
                is_primitive_type(inner_ty)
            } else {
                false
            }
        })
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            quote! { self.#name }
        })
        .collect();

    let non_option_field_names: Vec<_> = fields
        .iter()
        .filter(|f| {
            if f.flatten || f.skip {
                return false;
            }
            // Only include primitive types that can be handled by Clap
            is_option_type(&f.ty).is_none() && is_primitive_type(&f.ty)
        })
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            name.to_string()
        })
        .collect();

    let non_option_field_access: Vec<_> = fields
        .iter()
        .filter(|f| {
            if f.flatten || f.skip {
                return false;
            }
            // Only include primitive types that can be handled by Clap
            is_option_type(&f.ty).is_none() && is_primitive_type(&f.ty)
        })
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            quote! { self.#name }
        })
        .collect();

    // 收集展平的字段信息用于 to_cli_args
    let flattened_fields_info: Vec<_> = fields
        .iter()
        .filter(|f| f.flatten && !f.skip)
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            let name_str = name.to_string();
            let is_serde_flattened = has_serde_flatten(&f.attrs);

            if is_serde_flattened {
                quote! { (self.#name.to_cli_args(), None::<&str>) }
            } else {
                quote! { (self.#name.to_cli_args(), Some(#name_str)) }
            }
        })
        .collect();

    // 收集字段的自定义验证函数
    let custom_validations: Vec<_> = fields
        .iter()
        .filter(|f| !f.skip)
        .filter_map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();

            if f.flatten {
                return Some(quote!{
                    #[cfg(feature = "validation")]
                    confers::validator::Validate::validate(&config.#field_name)
                        .map_err(|e| confers::prelude::ConfigError::ValidationError(format!("验证失败: {:?}", e)))?;
                });
            }

            if let Some(validate_str) = &f.validate {
                if let Some((min, max)) = parse_range_validation(validate_str) {
                    let min_lit = syn::LitInt::new(&min.to_string(), proc_macro2::Span::call_site());
                    let max_lit = syn::LitInt::new(&max.to_string(), proc_macro2::Span::call_site());

                    return Some(quote!{
                        #[cfg(feature = "validation")]
                        if !(#min_lit as _..=#max_lit as _).contains(&config.#field_name) {
                            let mut errors = validator::ValidationErrors::new();
                             let mut error = validator::ValidationError::new("range");
                             error.message = Some(std::borrow::Cow::Owned(
                                 format!("value must be between {} and {}", #min_lit, #max_lit)
                             ));
                             error.add_param(std::borrow::Cow::Borrowed("min"), &#min_lit);
                             error.add_param(std::borrow::Cow::Borrowed("max"), &#max_lit);
                             error.add_param(std::borrow::Cow::Borrowed("value"), &config.#field_name);
                             errors.add(#field_name_str, error);
                             let error_msg = format!("验证失败: {:?}", errors);
                             return Err(confers::prelude::ConfigError::ValidationError(error_msg));
                        }
                    });
                }
            }

            if let Some(validate_str) = &f.validate {
                if let Some((min, max)) = parse_length_validation(validate_str) {
                    let min_lit = syn::LitInt::new(&min.to_string(), proc_macro2::Span::call_site());
                    let max_lit = syn::LitInt::new(&max.to_string(), proc_macro2::Span::call_site());

                    return Some(quote!{
                        #[cfg(feature = "validation")]
                        let field_len = config.#field_name.chars().count() as u64;
                        if !(#min_lit..=#max_lit).contains(&field_len) {
                            let mut errors = validator::ValidationErrors::new();
                            let mut error = validator::ValidationError::new("length");
                            error.message = Some(std::borrow::Cow::Owned(
                                format!("length must be between {} and {}", #min_lit, #max_lit)
                            ));
                            error.add_param(std::borrow::Cow::Borrowed("min"), &#min_lit);
                            error.add_param(std::borrow::Cow::Borrowed("max"), &#max_lit);
                            error.add_param(std::borrow::Cow::Borrowed("value"), &field_len);
                            errors.add(#field_name_str, error);
                            let error_msg = format!("验证失败: {:?}", errors);
                            return Err(confers::prelude::ConfigError::ValidationError(error_msg));
                        }
                    });
                }
            }

            if let Some(validate_str) = &f.validate {
                if validate_str == "email" {
                    return Some(quote!{
                        #[cfg(feature = "validation")]
                        if !confers::validators::is_email(&config.#field_name) {
                            let mut errors = validator::ValidationErrors::new();
                            let mut error = validator::ValidationError::new("email");
                            error.message = Some(std::borrow::Cow::Owned(
                                "must be a valid email address".to_string()
                            ));
                            error.add_param(std::borrow::Cow::Borrowed("value"), &config.#field_name);
                            errors.add(#field_name_str, error);
                            let error_msg = format!("验证失败: {:?}", errors);
                            return Err(confers::prelude::ConfigError::ValidationError(error_msg));
                        }
                    });
                }
            }

            if let Some(validate_str) = &f.validate {
                if validate_str == "url" {
                    return Some(quote!{
                        #[cfg(feature = "validation")]
                        if !confers::validators::is_url(&config.#field_name) {
                            let mut errors = validator::ValidationErrors::new();
                            let mut error = validator::ValidationError::new("url");
                            error.message = Some(std::borrow::Cow::Owned(
                                "must be a valid URL".to_string()
                            ));
                            error.add_param(std::borrow::Cow::Borrowed("value"), &config.#field_name);
                            errors.add(#field_name_str, error);
                            let error_msg = format!("验证失败: {:?}", errors);
                            return Err(confers::prelude::ConfigError::ValidationError(error_msg));
                        }
                    });
                }
            }

            let validation_fn = get_custom_validator(f)?;
            let validation_fn_path: syn::Path = syn::parse_str(&validation_fn).ok()?;

            Some(quote!{
                #validation_fn_path(&config.#field_name)
                    .map_err(|e| {
                        let mut errors = validator::ValidationErrors::new();
                        let mut error = validator::ValidationError::new("custom");
                        error.message = Some(std::borrow::Cow::Owned(format!("{}", e)));
                        error.add_param(std::borrow::Cow::Borrowed("value"), &config.#field_name);
                        errors.add(#field_name_str, error);
                        confers::prelude::ConfigError::ValidationError(format!("验证失败: {:?}", errors))
                    })?;
            })
        })
        .collect();

    let has_field_validations = fields.iter().any(|f| {
        !f.skip
            && (f.validate.is_some()
                || f.custom_validate.is_some()
                || !f.attrs.is_empty()
                || f.flatten)
    });

    let has_validations = opts.validate.as_ref().map(|v| v.0).unwrap_or(false)
        || has_validate_derive
        || has_field_validations;

    // Generate schemars implementation if the crate is available
    let schema_impl = {
        let schema_fields = fields.iter().filter(|f| !f.skip).map(|f| {
            let name = f.ident.as_ref().unwrap();
            let name_str = name.to_string();
            let ty = &f.ty;
            let description = &f.description;
            let default_expr = &f.default;

            let desc_str = if let Some(desc) = description {
                 if let Some(d) = default_expr {
                     let d_str = quote!(#d).to_string();
                     format!("{} (Default: {})", desc, d_str)
                 } else {
                     desc.clone()
                 }
            } else if let Some(d) = default_expr {
                 let d_str = quote!(#d).to_string();
                 format!("(Default: {})", d_str)
            } else {
                 String::new()
            };

            let default_quote = if let Some(d) = default_expr {
                let d_str = quote!(#d).to_string();
                let json_value = if d_str.starts_with('"') && d_str.ends_with('"') {
                    let trimmed = &d_str[1..d_str.len()-1];
                    quote! {
                        serde_json::Value::String(#trimmed.to_string())
                    }
                } else if d_str.parse::<u64>().is_ok() {
                    let n = d_str.parse::<u64>().unwrap();
                    quote! { serde_json::Value::Number(#n.into()) }
                } else if d_str.parse::<i64>().is_ok() {
                    let n = d_str.parse::<i64>().unwrap();
                    quote! { serde_json::Value::Number(#n.into()) }
                } else if d_str == "true" {
                    quote! { serde_json::Value::Bool(true) }
                } else if d_str == "false" {
                    quote! { serde_json::Value::Bool(false) }
                } else {
                    quote! { serde_json::Value::String(#d_str.to_string()) }
                };
                quote!{
                    let default_val = #json_value;
                    if let Some(metadata) = field_schema_obj.metadata.as_mut() {
                        metadata.default = Some(default_val);
                    } else {
                        let mut new_metadata = schemars::schema::Metadata::default();
                        new_metadata.default = Some(default_val);
                        field_schema_obj.metadata = Some(Box::new(new_metadata));
                    }
                }
            } else {
                quote! {}
            };

            let desc_quote = if !desc_str.is_empty() {
                quote!{
                    if let Some(metadata) = field_schema_obj.metadata.as_mut() {
                        metadata.description = Some(#desc_str.to_string());
                    } else {
                        let mut new_metadata = schemars::schema::Metadata::default();
                        new_metadata.description = Some(#desc_str.to_string());
                        field_schema_obj.metadata = Some(Box::new(new_metadata));
                    }
                }
            } else {
                quote! {}
            };

            if f.flatten {
                 quote!{
                     let sub_schema = <#ty as schemars::JsonSchema>::json_schema(gen);
                     if let schemars::schema::Schema::Object(mut obj) = sub_schema {
                         if let Some(object_validation) = &mut obj.object {
                             let obj_props = std::mem::take(&mut object_validation.properties);
                             let obj_required = std::mem::take(&mut object_validation.required);
                             for (k, v) in obj_props {
                                properties.insert(k, v);
                            }
                            for k in obj_required {
                                required.insert(k);
                            }
                         }
                     }
                 }
            } else {
                let is_optional = is_option_type(ty).is_some() || f.default.is_some();
                let required_quote = if !is_optional {
                    quote! { required.insert(#name_str.to_string()); }
                } else {
                    quote! {}
                };

                quote!{
                    let mut field_schema_obj = gen.subschema_for::<#ty>().into_object();
                    #desc_quote
                    #default_quote
                    properties.insert(#name_str.to_string(), schemars::schema::Schema::Object(field_schema_obj));
                    #required_quote
                }
            }
        });

        quote! {
            #[cfg(feature = "schema")]
            impl schemars::JsonSchema for #struct_name {
                fn schema_name() -> String {
                    stringify!(#struct_name).to_string()
                }

                fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                    use schemars::JsonSchema;

                    let mut schema_obj = schemars::schema::SchemaObject {
                        instance_type: Some(schemars::schema::InstanceType::Object.into()),
                        ..Default::default()
                    };

                    let mut properties = std::collections::BTreeMap::new();
                    let mut required = std::collections::BTreeSet::new();

                    #(#schema_fields)*

                    schema_obj.object = Some(Box::new(schemars::schema::ObjectValidation {
                        properties,
                        required,
                        additional_properties: None,
                        pattern_properties: std::collections::BTreeMap::new(),
                        max_properties: None,
                        min_properties: None,
                        property_names: None,
                    }));

                    schemars::schema::Schema::Object(schema_obj)
                }
            }
        }
    };

    let default_impl_body = fields.iter().map(|f| {
        let name = &f.ident;
        if let Some(d) = &f.default {
            let ty = &f.ty;
            let type_string = quote!(#ty).to_string();
            let is_str_type = is_string_type(ty);
            let is_str_lit = is_string_literal(d);
            if type_string.starts_with("Option <") {
                quote! { #name: Some(#d) }
            } else if is_str_type && is_str_lit {
                quote! { #name: #d.to_string() }
            } else {
                quote! { #name: #d }
            }
        } else {
            quote! { #name: Default::default() }
        }
    });

    let impl_default = quote! {
        impl Default for #struct_name {
            fn default() -> Self {
                Self {
                    #(#default_impl_body),*,
                }
            }
        }
    };

    let sanitize_body = fields.iter().map(|f| {
        let name = &f.ident;
        let name_str = name.as_ref().unwrap().to_string();
        let sensitive = f.sensitive.unwrap_or(false);

        if sensitive {
            quote!{
                let value = confers::serde_json::to_value(&self.#name).unwrap_or(confers::serde_json::Value::Null);
                let sanitized = confers::audit::sanitize_value(&value, &confers::audit::SanitizeConfig::default());
                map.insert(#name_str.to_string(), sanitized);
            }
        } else {
            quote!{
                if let Ok(val) = confers::serde_json::to_value(&self.#name) {
                    map.insert(#name_str.to_string(), val);
                }
            }
        }
    });

    let impl_sanitize = quote! {
        impl confers::audit::Sanitize for #struct_name {
            fn sanitize(&self) -> confers::serde_json::Value {
                let mut map = confers::serde_json::Map::new();
                #(#sanitize_body)*
                confers::serde_json::Value::Object(map)
            }
        }
    };

    let mut validation_fields = Vec::new();

    for field in fields {
        if let Some(field_ident) = &field.ident {
            let field_name_str = field_ident.to_string();

            if field.flatten {
                validation_fields.push(quote! {
                    confers::validator::Validate::validate(&self.#field_ident)?;
                });
            }

            if let Some(validate_str) = &field.validate {
                if let Some((min, max)) = parse_range_validation(validate_str) {
                    let min_lit =
                        syn::LitInt::new(&min.to_string(), proc_macro2::Span::call_site());
                    let max_lit =
                        syn::LitInt::new(&max.to_string(), proc_macro2::Span::call_site());
                    validation_fields.push(quote!{
                        if !(#min_lit as _..=#max_lit as _).contains(&self.#field_ident) {
                            let mut error = validator::ValidationError::new("range");
                            error.message = Some(std::borrow::Cow::Owned(
                                format!("value must be between {} and {}", #min_lit, #max_lit)
                            ));
                            error.add_param(std::borrow::Cow::Borrowed("min"), &#min_lit);
                            error.add_param(std::borrow::Cow::Borrowed("max"), &#max_lit);
                            error.add_param(std::borrow::Cow::Borrowed("value"), &self.#field_ident);
                            errors.add(#field_name_str, error);
                        }
                    });
                }

                if let Some((min, max)) = parse_length_validation(validate_str) {
                    let min_lit =
                        syn::LitInt::new(&min.to_string(), proc_macro2::Span::call_site());
                    let max_lit =
                        syn::LitInt::new(&max.to_string(), proc_macro2::Span::call_site());
                    validation_fields.push(quote! {
                        let field_len = self.#field_ident.chars().count() as u64;
                        if !(#min_lit..=#max_lit).contains(&field_len) {
                            let mut error = validator::ValidationError::new("length");
                            error.message = Some(std::borrow::Cow::Owned(
                                format!("length must be between {} and {}", #min_lit, #max_lit)
                            ));
                            error.add_param(std::borrow::Cow::Borrowed("min"), &#min_lit);
                            error.add_param(std::borrow::Cow::Borrowed("max"), &#max_lit);
                            error.add_param(std::borrow::Cow::Borrowed("value"), &field_len);
                            errors.add(#field_name_str, error);
                        }
                    });
                }

                if validate_str == "email" {
                    validation_fields.push(quote! {
                        if !confers::validators::is_email(&self.#field_ident) {
                            let mut error = validator::ValidationError::new("email");
                            error.message = Some(std::borrow::Cow::Owned(
                                "must be a valid email address".to_string()
                            ));
                            error.add_param(std::borrow::Cow::Borrowed("value"), &self.#field_ident);
                            errors.add(#field_name_str, error);
                        }
                    });
                }

                if let Some(validator_name) = validate_str.strip_prefix("custom:") {
                    let validator_name_lit =
                        syn::LitStr::new(validator_name, proc_macro2::Span::call_site());
                    validation_fields.push(quote! {
                        if !confers::validators::validate_with_custom(#validator_name_lit, &self.#field_ident) {
                            let mut error = validator::ValidationError::new("custom");
                            error.message = Some(std::borrow::Cow::Owned(
                                format!("failed custom validation: {}", #validator_name_lit)
                            ));
                            error.add_param(std::borrow::Cow::Borrowed("validator"), &#validator_name_lit);
                            error.add_param(std::borrow::Cow::Borrowed("value"), &self.#field_ident);
                            errors.add(#field_name_str, error);
                        }
                    });
                }
            }

            if field.custom_validate.is_some() {
                validation_fields.push(quote! {
                    if let Err(e) = self.#field_ident.validate() {
                        errors.add(#field_name_str, e);
                    }
                });
            }
        }
    }

    let impl_validate = quote! {
        #[cfg(feature = "validation")]
        impl validator::Validate for #struct_name {
            fn validate(&self) -> Result<(), validator::ValidationErrors> {
                let mut errors = validator::ValidationErrors::new();

                #(#validation_fields)*

                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
        }
    };

    // Generate ClapShadow struct name
    let clap_shadow_name = quote::format_ident!("{}ClapShadow", struct_name);

    // Generate field mapping for to_map method
    let to_map_fields = fields.iter().filter_map(|f| {
        // Skip flattened fields - they'll be handled separately
        if f.flatten || f.skip {
            return None;
        }

        let field_name = if let Some(name) = &f.name_config {
            name.clone()
        } else {
            f.ident.as_ref()?.to_string()
        };
        let field_ident = f.ident.as_ref()?;
        Some(quote! {
            if let Ok(value) = figment::value::Value::serialize(&self.#field_ident) {
                map.insert(#field_name.to_string(), value);
            }
        })
    });

    // Handle flattened fields separately
    let flattened_map_fields = fields.iter().filter(|f| f.flatten).map(|f| {
        let name = f.ident.as_ref().unwrap();
        let is_serde_flatten = has_serde_flatten(&f.attrs);

        if is_serde_flatten {
            // For serde(flatten) fields, create both nested and flat keys
            quote! {
                let field_name = stringify!(#name);
                let flattened_map = self.#name.to_map();
                for (k, v) in flattened_map {
                    // Add nested key (e.g., details.count)
                    map.insert(format!("{}.{}", field_name, k), v.clone());
                    // Add flat key for Figment double underscore (e.g., details_count)
                    map.insert(format!("{}_{}", field_name, k), v.clone());
                    // Add flat key for serde(flatten) (e.g., count)
                    map.insert(k, v);
                }
            }
        } else {
            // For regular flattened fields, only create nested and double underscore keys
            quote! {
                let field_name = stringify!(#name);
                let flattened_map = self.#name.to_map();
                for (k, v) in flattened_map {
                    map.insert(format!("{}.{}", field_name, k), v.clone());
                    map.insert(format!("{}_{}", field_name, k), v);
                }
            }
        }
    });

    // Generate env mapping
    let env_mapping_entries = fields.iter().filter_map(|f| {
        if f.flatten || f.skip {
            return None;
        }

        if let Some(env_name) = &f.name_env {
            let field_ident = f.ident.as_ref()?;
            let field_name = field_ident.to_string();
            Some(quote! {
                map.insert(#field_name.to_string(), #env_name.to_string());
            })
        } else {
            None
        }
    });

    let impl_optional_validate = quote! {
        #[cfg(not(feature = "validation"))]
        impl confers::OptionalValidate for #struct_name {}
    };

    let generated_code = quote! {
        #impl_default
        #impl_sanitize
        #impl_validate
        #impl_optional_validate
        #schema_impl

        /// Clap-compatible argument structure for command line parsing
        #[cfg(feature = "cli")]
        #[derive(confers::clap::Parser, Debug, serde::Serialize, serde::Deserialize)]
        #[clap(name = stringify!(#struct_name))]
        struct #clap_shadow_name {
            #(#clap_fields)*
        }

        impl #struct_name {
            /// Create a new ConfigLoader for this struct
            pub fn new_loader() -> confers::core::ConfigLoader<Self> {
                let mut loader = confers::core::ConfigLoader::<Self>::new();

                // Only set app_name if explicitly configured via #[config(app_name = "...")]
                // Otherwise, the default search path will include the current directory
                #apply_app_name

                if !#env_prefix.is_empty() {
                    loader = loader.with_env_prefix(#env_prefix);
                }
                if #strict {
                    loader = loader.with_strict(true);
                }
                #apply_watch
                #apply_format_detection
                #apply_remote
                #apply_remote_timeout
                #apply_remote_fallback
                #apply_remote_auth
                #apply_remote_tls
                #(#field_remote_configs)*
                #apply_audit
                #[cfg(test)]
                {
                    loader = loader.with_memory_limit(0);
                }
                #[cfg(not(test))]
                {
                    // Check if memory limit check should be disabled via environment variable
                    if std::env::var("CONFFERS_DISABLE_MEMORY_LIMIT").is_ok() ||
                       std::env::var("CONFFERS_MEMORY_LIMIT").map_or(false, |v| v == "0") {
                        loader = loader.with_memory_limit(0);
                    }
                }
                loader.with_defaults(Self::default())
            }

            /// Load configuration from multiple sources including command line arguments
            pub fn load() -> Result<Self, confers::prelude::ConfigError> {
                let mut loader = Self::new_loader();

                // Parse command line arguments and add them as overrides
                #[cfg(all(feature = "cli", not(test)))]
                match <#clap_shadow_name as confers::clap::Parser>::try_parse() {
                    Ok(clap_args) => {
                        // Convert Clap arguments to CLI format and add them
                        let cli_args = clap_args.to_cli_args();
                        loader = loader.with_cli_provider(confers::providers::cli_provider::CliConfigProvider::from_args(cli_args));
                    }
                    Err(e) => {
                        match e.kind() {
                            confers::clap::error::ErrorKind::DisplayHelp | confers::clap::error::ErrorKind::DisplayVersion => {
                                e.print().ok();
                                std::process::exit(0);
                            },
                            _ => {
                                if #strict {
                                    return Err(confers::prelude::ConfigError::from(format!("{}", e)));
                                } else {
                                }
                            }
                        }
                    }
                }
                #[cfg(all(feature = "cli", test))]
                {
                    let clap_args = <#clap_shadow_name as confers::clap::Parser>::try_parse_from(std::iter::empty::<&str>()).ok();
                    if let Some(clap_args) = clap_args {
                        let cli_args = clap_args.to_cli_args();
                        loader = loader.with_cli_provider(confers::providers::cli_provider::CliConfigProvider::from_args(cli_args));
                    }
                }

                // Load config
                #[cfg(feature = "audit")]
                let config = futures::executor::block_on(async { loader.load().await })?;
                #[cfg(not(feature = "audit"))]
                let config = futures::executor::block_on(async { loader.load().await })?;

                // Apply validation only if there are validations to apply
                #[cfg(feature = "validation")]
                if #has_validations {
                    confers::validator::Validate::validate(&config).map_err(|e| confers::prelude::ConfigError::ValidationError(format!("验证失败: {:?}", e)))?;
                }

                // Apply custom field validations
                #(#custom_validations)*

                Ok(config)
            }

            /// Load configuration with custom command line arguments
            #[cfg(feature = "cli")]
            pub fn load_from_args(args: Vec<String>) -> Result<Self, confers::prelude::ConfigError> {
                let mut loader = Self::new_loader();

                // Parse provided command line arguments
                match <#clap_shadow_name as confers::clap::Parser>::try_parse_from(args) {
                    Ok(clap_args) => {
                        // Convert Clap arguments to CLI format and add them
                        let cli_args = clap_args.to_cli_args();
                        loader = loader.with_cli_provider(confers::providers::cli_provider::CliConfigProvider::from_args(cli_args));
                    }
                    Err(e) => {
                        match e.kind() {
                            confers::clap::error::ErrorKind::DisplayHelp | confers::clap::error::ErrorKind::DisplayVersion => {
                                e.print().ok();
                                std::process::exit(0);
                            },
                            _ => {
                                if #strict {
                                    return Err(confers::prelude::ConfigError::from(format!("{}", e)));
                                } else {
                                    // Ignore CLI parsing errors for optional args in relaxed mode
                                    // eprintln!("Warning: Failed to parse CLI arguments: {}", e);
                                }
                            }
                        }
                    }
                }

                // Load config
                #[cfg(feature = "audit")]
                let config = futures::executor::block_on(async { loader.load().await })?;
                #[cfg(not(feature = "audit"))]
                let config = futures::executor::block_on(async { loader.load().await })?;

                // Apply validation only if there are validations to apply
                #[cfg(feature = "validation")]
                if #has_validations {
                    confers::validator::Validate::validate(&config).map_err(|e| confers::prelude::ConfigError::ValidationError(format!("验证失败: {:?}", e)))?;
                }

                // Apply custom field validations
                #(#custom_validations)*

                Ok(config)
            }

            /// Load configuration and return it along with an optional watcher
            #[cfg(feature = "watch")]
            pub fn load_with_watcher() -> Result<(Self, Option<confers::watcher::ConfigWatcher>), confers::prelude::ConfigError> {
                Self::new_loader().load_sync_with_watcher()
            }

            /// Load configuration with strict mode enabled/disabled
            pub fn load_with_strict(strict: bool) -> confers::core::ConfigLoader<Self> {
                let mut loader = Self::new_loader();
                if strict {
                    loader = loader.with_strict(true);
                }
                loader
            }

            /// Create a loader for this configuration
            pub fn load_file(path: impl Into<std::path::PathBuf>) -> confers::core::ConfigLoader::<Self> {
                let mut loader = Self::new_loader();

                // Add the explicit file
                let path_buf = path.into();
                loader = loader.with_file(path_buf.as_path());

                loader
            }

            /// Load configuration synchronously
            pub fn load_sync() -> Result<Self, confers::prelude::ConfigError>
            where
                Self: Sized + confers::Sanitize + for<'de> serde::Deserialize<'de> + serde::Serialize + std::default::Default + Clone + confers::ConfigMap,
            {
                let mut loader = Self::new_loader();
                let config = futures::executor::block_on(async { loader.load().await })?;

                // Apply validation only if there are validations to apply
                #[cfg(feature = "validation")]
                if #has_validations {
                    confers::validator::Validate::validate(&config).map_err(|e| confers::prelude::ConfigError::ValidationError(format!("验证失败: {:?}", e)))?;
                }

                // Apply custom field validations
                #(#custom_validations)*

                Ok(config)
            }

            /// Convert this configuration to a map of key-value pairs for Figment serialization
            pub fn to_map(&self) -> std::collections::HashMap<String, figment::value::Value> {
                let mut map = std::collections::HashMap::new();

                // Add regular fields
                #(#to_map_fields)*

                // Add flattened fields
                #(#flattened_map_fields)*

                map
            }

            /// Generate JSON Schema for this configuration type
            #[cfg(feature = "schema")]
            pub fn json_schema() -> serde_json::Value {
                let schema = schemars::schema_for!(#struct_name);
                serde_json::to_value(schema).expect("Failed to convert schema to JSON")
            }

            /// Generate TypeScript schema definition for this configuration type
            #[cfg(feature = "schema")]
            pub fn typescript_schema() -> String {
                confers::schema::typescript::TypeScriptGenerator::generate::<#struct_name>()
            }

            /// Export JSON schema to a file
            #[cfg(feature = "schema")]
            pub fn export_schema(path: impl AsRef<std::path::Path>) -> Result<(), confers::prelude::ConfigError> {
                let schema = Self::json_schema();
                let schema_json = serde_json::to_string_pretty(&schema)
                    .map_err(|e| confers::prelude::ConfigError::SerializationError(e.to_string()))?;

                std::fs::write(path, schema_json)
                    .map_err(|e| confers::prelude::ConfigError::IoError(e.to_string()))?;

                Ok(())
            }
        }

        impl confers::ConfigMap for #struct_name {
            fn to_map(&self) -> std::collections::HashMap<String, confers::serde_json::Value> {
                confers::serde_json::to_value(self)
                    .map(|v| {
                        if let confers::serde_json::Value::Object(map) = v {
                            map.into_iter().collect()
                        } else {
                            std::collections::HashMap::new()
                        }
                    })
                    .unwrap_or_default()
            }

            fn env_mapping() -> std::collections::HashMap<String, String> {
                let mut map = std::collections::HashMap::new();
                #(#env_mapping_entries)*
                map
            }
        }

        /// Convert ClapShadow to CLI arguments in key=value format
        #[cfg(feature = "cli")]
        impl #clap_shadow_name {
            pub fn to_cli_args(&self) -> Vec<String> {
                let mut args = Vec::new();
                #(
                    // Convert flattened fields
                    let (sub_args, prefix) = #flattened_fields_info;
                    for arg in sub_args {
                         if let Some(p) = prefix {
                             args.push(format!("{}.{}", p, arg));
                         } else {
                             args.push(arg);
                         }
                    }
                )*

                #(
                    // Process Option<T> fields - only include if Some(value)
                    let field_name = #option_field_names;
                    let field_value = &#option_field_access;

                    // For Option<T> fields, only include if it's Some(value)
                    if let Some(value) = field_value {
                        if let Ok(value_str) = confers::serde_json::to_string(value) {
                            // Remove quotes from string values
                            let clean_value = if value_str.starts_with('"') && value_str.ends_with('"') {
                                value_str[1..value_str.len()-1].to_string()
                            } else {
                                value_str
                            };
                            args.push(format!("{}={}", field_name, clean_value));
                        }
                    }
                )*

                #(
                    // Process non-Option fields - but they're wrapped in Option<T> by ClapShadow
                    let field_name = #non_option_field_names;
                    let field_value = &#non_option_field_access;

                    // Since ClapShadow wraps non-Option fields in Option<T>, we need to check if it's Some
                    if let Some(value) = field_value {
                        if let Ok(value_str) = confers::serde_json::to_string(value) {
                            // Remove quotes from string values
                            let clean_value = if value_str.starts_with('"') && value_str.ends_with('"') {
                                value_str[1..value_str.len()-1].to_string()
                            } else {
                                value_str
                            };
                            args.push(format!("{}={}", field_name, clean_value));
                        }
                    }
                    // If it's None, skip this field entirely - don't generate "field=null"
                )*
                args
            }
        }
    }; // Close the quote! block

    generated_code
}
