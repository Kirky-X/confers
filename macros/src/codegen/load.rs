// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Load method generation for Config derive macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::parse::{DefaultExpr, FieldAttrs, StructAttrs, parse_field_attrs};

/// Generate the load methods for a struct.
pub fn generate_load_impl(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &syn::Fields,
) -> TokenStream {
    let env_prefix = attrs.effective_env_prefix();

    // Field information; malformed `#[config(...)]` attributes surface as
    // compile errors appended to the output instead of dropping the field.
    let (field_info, attr_errors) = parse_field_attrs(fields);

    // Generate load() method
    let load_impl = generate_load_method(struct_ident, attrs, &field_info);

    // generate load_sync() method
    let load_sync_impl = generate_load_sync_method(struct_ident, attrs, &field_info);

    // Generate load_file() method
    let load_file_impl = generate_load_file_method(struct_ident, attrs, &field_info);

    // Generate env_mapping() method
    let env_mapping_impl = generate_env_mapping(struct_ident, env_prefix, &field_info);

    quote! {
        #attr_errors
        #load_impl
        #load_sync_impl
        #load_file_impl
        #env_mapping_impl
    }
}

/// Generate the default source registration statements.
///
/// Defaults are registered under the **serde field name** — the merged value
/// tree (file keys included) is keyed by serde field names, so a default
/// parked under a renamed config key would never reach deserialization.
/// `skip` fields with a `default` attribute are registered too: the value is
/// needed for deserialization itself, and the post-build materialization then
/// makes it immune to any source override.
fn generate_default_calls(fields: &[(&syn::Ident, &syn::Type, FieldAttrs)]) -> Vec<TokenStream> {
    fields
        .iter()
        .filter(|(_, _, f)| f.default.is_some())
        .map(|(_, ty, f)| {
            let field_key = f.serde_name();
            let default_expr = f.default.as_ref().unwrap();
            // `default = None` serializes as the null ConfigValue, which
            // deserializes back to `None` for Option fields; the bare-word
            // form pulls the field type's `Default` impl.
            let value_init = if default_expr.is_none() {
                quote! { confers::ConfigValue::Null }
            } else {
                match default_expr {
                    DefaultExpr::TypeDefault => {
                        quote! { <#ty as ::std::default::Default>::default().into() }
                    }
                    DefaultExpr::Expr(expr) => quote! { (#expr).into() },
                }
            };

            quote! {
                builder = builder.default(#field_key.to_string(), {
                    let val: confers::ConfigValue = #value_init;
                    val
                });
            }
        })
        .collect()
}

/// Generate the env extraction statements filling `env_map`.
///
/// Each entry is keyed by the **serde field name** (the merge space used by
/// the file source and deserialization) while the env var name follows
/// `name_env` when declared (verbatim, no default-naming fallback) or the
/// default `PREFIX+KEY` upper-case rule otherwise. `skip` fields are excluded:
/// they never participate in loading.
fn generate_env_calls(
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
    env_prefix: &str,
) -> Vec<TokenStream> {
    fields
        .iter()
        .filter(|(_, _, f)| !f.skip)
        .map(|(_, _, f)| {
            let env_name = f.effective_env_name(env_prefix);
            let field_key = f.serde_name();

            // Handle _FILE suffix for secrets with secure path validation
            if f.is_sensitive_effective() {
                let file_env_name = format!("{}_FILE", env_name);
                quote! {
                    // Check for _FILE suffix first (Docker/K8s secrets pattern).
                    // Security: PathValidator prevents directory traversal
                    // attacks; invalid or unreadable secret files are hard
                    // errors (never silently skipped, matching EnvSource).
                    if let Ok(file_path) = std::env::var(#file_env_name) {
                        let validator = confers::PathValidator::new();
                        let validated_path = validator.validate_and_resolve(&file_path)?;
                        let content = std::fs::read_to_string(&validated_path).map_err(|_| {
                            confers::ConfigError::InvalidValue {
                                key: #file_env_name.to_string(),
                                expected_type: "readable file".to_string(),
                                message: ::std::format!(
                                    "Cannot read file referenced by {}",
                                    #file_env_name
                                ),
                            }
                        })?;
                        let val = content.trim().to_string();
                        env_map.insert(#field_key.to_string(), confers::EnvSource::infer_config_value(&val));
                    } else if let Ok(val) = std::env::var(#env_name) {
                        env_map.insert(#field_key.to_string(), confers::EnvSource::infer_config_value(&val));
                    }
                }
            } else {
                quote! {
                    if let Ok(val) = std::env::var(#env_name) {
                        env_map.insert(#field_key.to_string(), confers::EnvSource::infer_config_value(&val));
                    }
                }
            }
        })
        .collect()
}

/// Generate the statements that force-materialize `skip` fields after a
/// successful build.
///
/// `skip` fields never participate in loading (no env entry, no file key may
/// reach them), so their value is assigned last — from the `default` attribute
/// when present, otherwise the field type's `Default::default()`. This runs
/// after deserialization, making the skip default the final word over every
/// source.
fn generate_skip_materialization(
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> Vec<TokenStream> {
    fields
        .iter()
        .filter(|(_, _, f)| f.skip)
        .map(|(ident, _, f)| {
            let init = match f.default.as_ref() {
                Some(DefaultExpr::TypeDefault) => quote! { ::std::default::Default::default() },
                Some(e) if e.is_none() => quote! { None },
                Some(DefaultExpr::Expr(expr)) => quote! { #expr },
                None => quote! { ::std::default::Default::default() },
            };
            quote! { config.#ident = #init; }
        })
        .collect()
}

/// Naming styles accepted by `#[config(rename_all = "...")]`.
///
/// Convert a serde field name into the external (file) key form. The derive
/// then generates a `rename_tree_keys` pass that maps the external form back
/// to the serde name right before deserialization.
fn to_camel_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut uppercase_next = false;
    for ch in name.chars() {
        if ch == '_' {
            uppercase_next = true;
        } else if uppercase_next {
            out.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn to_kebab_case(name: &str) -> String {
    name.replace('_', "-")
}

/// External (file) key for `name` under `rename_all`; `None` keeps the serde
/// field name (identity mapping is skipped by the runtime pass).
fn external_key(rename_all: &str, name: &str) -> String {
    match rename_all {
        "camelCase" => to_camel_case(name),
        "kebab-case" => to_kebab_case(name),
        _ => name.to_string(),
    }
}

/// Generate a `builder.map_json(...)` pass renaming the struct's own keys
/// from the configured batch style back to serde field names. Returns an
/// empty stream when `rename_all` is not configured.
fn generate_rename_all_call(
    rename_all: Option<&String>,
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> Option<TokenStream> {
    let style = rename_all?;
    let mappings: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip)
        .map(|(_, _, f)| {
            let serde_name = f.serde_name();
            let external = external_key(style, &serde_name);
            quote! { (#external, #serde_name) }
        })
        .collect();
    Some(quote! {
        builder = builder.map_json(|json: &mut confers::json::Value| {
            confers::rename_tree_keys(json, &[#(#mappings),*]);
        });
    })
}

/// Generate `builder.map_json(...)` registrations for the `flatten` and
/// `interpolate` field attributes.
///
/// Both transforms run on the merged JSON tree right before deserialization:
/// - `flatten` hoists top-level keys addressed to flattened nested structs
///   into `field_key.nested_key` form (explicit parent/nested values win).
/// - `interpolate` resolves `${key}` / `${key:default}` references inside the
///   marked fields against the merged tree.
///
/// Transforms compose in registration order (flatten before interpolate).
/// The returned statements assume a `builder` variable in scope.
fn generate_map_json_calls(fields: &[(&syn::Ident, &syn::Type, FieldAttrs)]) -> Vec<TokenStream> {
    let mut calls = Vec::new();

    // `flatten`: one combined pass with every flattened field's spec.
    let own_keys: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip)
        .map(|(_, _, f)| {
            let key = f.serde_name();
            quote! { #key }
        })
        .collect();
    let flatten_specs: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| f.flatten)
        .map(|(_, ty, f)| {
            let field_key = f.serde_name();
            quote! {
                confers::FlattenSpec {
                    field_key: #field_key,
                    nested_keys: <#ty as confers::ConfigFieldKeys>::FIELD_KEYS,
                }
            }
        })
        .collect();
    if !flatten_specs.is_empty() {
        calls.push(quote! {
            builder = builder.map_json(|json: &mut confers::json::Value| {
                confers::hoist_flattened(json, &[#(#own_keys),*], &[#(#flatten_specs),*]);
            });
        });
    }

    // `interpolate`: one combined pass over every interpolated field.
    //
    // Keys are the **serde field names** (the merge/deserialization key
    // space), so `#[serde(rename)]`-renamed fields still interpolate.
    // Sensitive field keys are forwarded so referencing a sensitive value
    // from a non-sensitive template emits a warning event.
    let interpolate_keys: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| f.interpolate)
        .map(|(_, _, f)| {
            let key = f.serde_name();
            quote! { #key }
        })
        .collect();
    if !interpolate_keys.is_empty() {
        let sensitive_keys: Vec<TokenStream> = fields
            .iter()
            .filter(|(_, _, f)| !f.skip && f.is_sensitive_effective())
            .map(|(_, _, f)| {
                let key = f.serde_name();
                quote! { #key }
            })
            .collect();
        calls.push(quote! {
            builder = builder.map_json(|json: &mut confers::json::Value| {
                confers::interpolate_keys_with_sensitivity(
                    json,
                    &[#(#interpolate_keys),*],
                    &[#(#sensitive_keys),*],
                );
            });
        });
    }

    calls
}

/// Whether any non-skipped field declares `encrypt` (drives the
/// builder-level decryption pass + the decrypt marker).
fn has_encrypt_fields(fields: &[(&syn::Ident, &syn::Type, FieldAttrs)]) -> bool {
    fields
        .iter()
        .any(|(_, _, f)| f.encrypt.is_some() && !f.skip)
}

/// Shared body of every generated loader: defaults first (lowest priority),
/// then the declared env overrides as a memory source, then `finish` runs the
/// builder to a result and skip fields are materialized last.
fn loader_body(
    attrs: &StructAttrs,
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
    finish: TokenStream,
) -> TokenStream {
    let default_calls = generate_default_calls(fields);
    let env_calls = generate_env_calls(fields, attrs.effective_env_prefix());
    let skip_assigns = generate_skip_materialization(fields);
    // rename_all runs before the other tree transforms so flatten/interpolate
    // observe serde-normalized keys.
    let rename_call = generate_rename_all_call(attrs.rename_all.as_ref(), fields);
    let map_json_calls = generate_map_json_calls(fields);
    // 加密标记(builder 在反序列化前集中解密);解密必须跑在
    // 全部 tree transform 之后,故不能进 map_json_calls 的注册序。
    let decrypt_mark = if has_encrypt_fields(fields) {
        quote! { builder = builder.encrypted_fields(); }
    } else {
        quote! {}
    };
    let mut_kw = if skip_assigns.is_empty() {
        quote! {}
    } else {
        quote! { mut }
    };

    quote! {
        let mut builder = confers::ConfigBuilder::<Self>::new();
        // 快照脱敏所需的敏感路径(无敏感字段时为空)。
        builder = builder.sensitive_paths(Self::sensitive_paths());
        #decrypt_mark

        // Add defaults first (lowest priority)
        #(#default_calls)*

        // Add environment variables (higher priority)
        let mut env_map = std::collections::HashMap::new();
        #(#env_calls)*
        if !env_map.is_empty() {
            builder = builder.memory(env_map);
        }

        // Field-attribute transforms (rename_all first, then
        // flatten/interpolate) on the merged tree
        #rename_call
        #(#map_json_calls)*

        let #mut_kw config: Self = #finish?;
        // `skip` fields are the final word: no source may have set them.
        #(#skip_assigns)*
        Ok(config)
    }
}

/// Generate the async load() method
fn generate_load_method(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> TokenStream {
    let body = loader_body(attrs, fields, quote! { builder.build() });

    quote! {
        impl #struct_ident {
            /// Load configuration from environment variables and defaults.
            ///
            /// Priority order: declared defaults first (lowest), then the
            /// struct's environment variables (highest). Configuration files
            /// are not consulted — use [`Self::load_file`] for that.
            pub fn load() -> impl std::future::Future<Output = confers::ConfigResult<Self>> {
                async {
                    Self::load_sync()
                }
            }

            /// Load configuration synchronously.
            pub fn load_sync() -> confers::ConfigResult<Self> {
                #body
            }
        }
    }
}

/// Generate the synchronous load_sync() method
fn generate_load_sync_method(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> TokenStream {
    let body = loader_body(attrs, fields, quote! { builder.build() });

    quote! {
        impl #struct_ident {
            /// Build configuration with environment variables and defaults.
            pub fn build_config() -> confers::ConfigResult<Self> {
                #body
            }
        }
    }
}

/// Generate the load_file() method
fn generate_load_file_method(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> TokenStream {
    // Both file loaders register every field default first: without them a
    // partial file makes deserialization fail with `missing field` even when
    // the struct declares `#[config(default)]` for the absent keys.
    let default_calls = generate_default_calls(fields);
    let default_calls_in_env_loader = default_calls.clone();
    // load_file_with_env must carry the same declared-env overrides as
    // load_sync: the generic env source alone cannot honor `name_env`
    // declarations (it only maps UPPER_SNAKE → lower.dot paths). Only the
    // declared variables are injected — no blanket process-env source, which
    // would leak PATH/HOME into the merge tree and break
    // `deny_unknown_fields` structs with an empty key from `_`.
    let env_calls = generate_env_calls(fields, attrs.effective_env_prefix());
    let skip_assigns = generate_skip_materialization(fields);
    let skip_assigns_in_env_loader = skip_assigns.clone();
    let decrypt_mark = if has_encrypt_fields(fields) {
        Some(quote! { builder = builder.encrypted_fields(); })
    } else {
        None
    };
    let map_json_calls = generate_map_json_calls(fields);
    let map_json_calls_in_env_loader = generate_map_json_calls(fields);
    let rename_call = generate_rename_all_call(attrs.rename_all.as_ref(), fields);
    let rename_call_in_env_loader = rename_call.clone();
    let mut_kw = if skip_assigns.is_empty() {
        quote! {}
    } else {
        quote! { mut }
    };
    // `#[config(profile)]`: when the profile env var (default RUN_ENV) is
    // set to a non-empty value, `<stem>.<env>.<ext>` next to the base file is
    // loaded after it (later declaration wins), enabling per-environment
    // overlays. A missing overlay file is skipped silently.
    let (overlay_setup, overlay_apply) = if attrs.profile {
        let env_var = attrs.profile_env.as_deref().unwrap_or("RUN_ENV");
        (
            quote! {
                let overlay_path: Option<std::path::PathBuf> = {
                    let env_name = std::env::var(#env_var).unwrap_or_default();
                    if env_name.is_empty() {
                        None
                    } else {
                        let base = path.as_ref();
                        match (
                            base.file_stem().and_then(|s| s.to_str()),
                            base.extension().and_then(|s| s.to_str()),
                        ) {
                            (Some(stem), Some(ext)) => {
                                Some(base.with_file_name(format!("{stem}.{env_name}.{ext}")))
                            }
                            _ => None,
                        }
                    }
                };
            },
            quote! {
                if let Some(overlay) = overlay_path {
                    if overlay.exists() {
                        builder = builder.file(overlay);
                    }
                }
            },
        )
    } else {
        (quote! {}, quote! {})
    };
    let file_builder_mut = if default_calls.is_empty()
        && map_json_calls.is_empty()
        && rename_call.is_none()
        && decrypt_mark.is_none()
        && !attrs.profile
    {
        quote! {}
    } else {
        quote! { mut }
    };
    let env_builder_mut = if default_calls_in_env_loader.is_empty()
        && env_calls.is_empty()
        && map_json_calls_in_env_loader.is_empty()
        && rename_call_in_env_loader.is_none()
        && decrypt_mark.is_none()
        && !attrs.profile
    {
        quote! {}
    } else {
        quote! { mut }
    };

    quote! {
        impl #struct_ident {
            /// Load configuration from a specific file.
            ///
            /// Priority: declared defaults (lowest) < file values.
            pub fn load_file(path: impl AsRef<std::path::Path>) -> confers::ConfigResult<Self> {
                #overlay_setup
                let #file_builder_mut builder = confers::ConfigBuilder::<Self>::new()
                    .file(path.as_ref())
                    .sensitive_paths(Self::sensitive_paths());
                #decrypt_mark
                #overlay_apply
                #(#default_calls)*
                // Field-attribute transforms (rename_all first, then flatten/interpolate)
                #rename_call
                #(#map_json_calls)*
                let #mut_kw config: Self = builder.build()?;
                // `skip` fields are the final word: no source may have set them.
                #(#skip_assigns)*
                Ok(config)
            }

            /// Load configuration from a specific file with environment overrides.
            ///
            /// Priority: declared defaults (lowest) < file values < declared
            /// environment variables (highest). Only variables matching the
            /// struct's declared env names are consulted.
            pub fn load_file_with_env(path: impl AsRef<std::path::Path>) -> confers::ConfigResult<Self> {
                #overlay_setup
                let #env_builder_mut builder = confers::ConfigBuilder::<Self>::new()
                    .file(path.as_ref())
                    .sensitive_paths(Self::sensitive_paths());
                #decrypt_mark
                #overlay_apply
                #(#default_calls_in_env_loader)*

                let mut env_map = std::collections::HashMap::new();
                #(#env_calls)*
                if !env_map.is_empty() {
                    builder = builder.memory(env_map);
                }

                // Field-attribute transforms (rename_all first, then flatten/interpolate)
                #rename_call_in_env_loader
                #(#map_json_calls_in_env_loader)*

                let #mut_kw config: Self = builder.build()?;
                // `skip` fields are the final word: no source may have set them.
                #(#skip_assigns_in_env_loader)*
                Ok(config)
            }
        }
    }
}

/// Generate the env_mapping() method
fn generate_env_mapping(
    struct_ident: &Ident,
    env_prefix: &str,
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> TokenStream {
    let mappings: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip)
        .map(|(ident, _, f)| {
            let config_key = f.effective_name();
            let env_name = f.effective_env_name(env_prefix);
            let field_name = ident.to_string();

            quote! {
                (#field_name.to_string(), #config_key.to_string(), #env_name.to_string())
            }
        })
        .collect();

    quote! {
        impl #struct_ident {
            /// Get the mapping of field names to configuration keys and environment variables.
            pub fn env_mapping() -> Vec<(String, String, String)> {
                vec![
                    #(#mappings),*
                ]
            }
        }
    }
}
