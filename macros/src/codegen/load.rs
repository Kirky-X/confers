// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Load method generation for Config derive macro.

use darling::FromField;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, Ident};

use crate::parse::{FieldAttrs, StructAttrs};

/// Generate the load methods for a struct.
pub fn generate_load_impl(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &syn::Fields,
) -> TokenStream {
    let env_prefix = attrs.effective_env_prefix();
    let named_fields = match fields {
        Fields::Named(named) => &named.named,
        _ => return quote! {},
    };

    // Collect field information
    let field_info: Vec<(&syn::Ident, &syn::Type, FieldAttrs)> = named_fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let attrs = FieldAttrs::from_field(field).ok()?;
            Some((ident, &field.ty, attrs))
        })
        .collect();

    // Generate load() method
    let load_impl = generate_load_method(struct_ident, attrs, &field_info);

    // generate load_sync() method
    let load_sync_impl = generate_load_sync_method(struct_ident, attrs, &field_info);

    // Generate load_file() method
    let load_file_impl = generate_load_file_method(struct_ident, attrs, &field_info);

    // Generate env_mapping() method
    let env_mapping_impl = generate_env_mapping(struct_ident, env_prefix, &field_info);

    quote! {
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
        .map(|(ident, _, f)| {
            let field_key = ident.to_string();
            let default_expr = f.default.as_ref().unwrap();

            quote! {
                builder = builder.default(#field_key.to_string(), {
                    let val: confers::ConfigValue = (#default_expr).into();
                    val
                });
            }
        })
        .collect()
}

/// Generate default registration statements for `skip` fields only.
///
/// The generated `load_file`/`load_file_with_env` loaders never registered
/// struct defaults (pre-existing semantics, kept untouched); a skipped field
/// carrying a `default` attribute is the one exception — without it the value
/// cannot deserialize when no source provides the key.
fn generate_skip_default_calls(
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> Vec<TokenStream> {
    fields
        .iter()
        .filter(|(_, _, f)| f.skip && f.default.is_some())
        .map(|(ident, _, f)| {
            let field_key = ident.to_string();
            let default_expr = f.default.as_ref().unwrap();

            quote! {
                builder = builder.default(#field_key.to_string(), {
                    let val: confers::ConfigValue = (#default_expr).into();
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
        .map(|(ident, _, f)| {
            let env_name = f.effective_env_name(env_prefix);
            let field_key = ident.to_string();

            // Handle _FILE suffix for secrets with secure path validation
            if f.is_sensitive_effective() {
                let file_env_name = format!("{}_FILE", env_name);
                quote! {
                    // Check for _FILE suffix first (Docker/K8s secrets pattern)
                    // Security: Use PathValidator to prevent directory traversal attacks
                    if let Ok(file_path) = std::env::var(#file_env_name) {
                        let validator = confers::security::PathValidator::new();
                        match validator.validate_and_resolve(&file_path) {
                            Ok(validated_path) => {
                                if let Ok(content) = std::fs::read_to_string(&validated_path) {
                                    let val = content.trim().to_string();
                                    env_map.insert(#field_key.to_string(), confers::EnvSource::infer_config_value(&val));
                                }
                            }
                            Err(_) => {
                                // Silently skip invalid secret file paths
                            }
                        }
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
                Some(expr) => quote! { #expr },
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
        .map(|(ident, _, _)| {
            let external = external_key(style, &ident.to_string());
            let serde_name = ident.to_string();
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
fn generate_map_json_calls(
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> Vec<TokenStream> {
    let mut calls = Vec::new();

    // `flatten`: one combined pass with every flattened field's spec.
    let own_keys: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip)
        .map(|(ident, _, _)| {
            let key = ident.to_string();
            quote! { #key }
        })
        .collect();
    let flatten_specs: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| f.flatten)
        .map(|(ident, ty, _)| {
            let field_key = ident.to_string();
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
    let interpolate_keys: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| f.interpolate)
        .map(|(ident, _, _)| {
            let key = ident.to_string();
            quote! { #key }
        })
        .collect();
    if !interpolate_keys.is_empty() {
        calls.push(quote! {
            builder = builder.map_json(|json: &mut confers::json::Value| {
                confers::interpolate_keys(json, &[#(#interpolate_keys),*]);
            });
        });
    }

    calls
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
    let mut_kw = if skip_assigns.is_empty() {
        quote! {}
    } else {
        quote! { mut }
    };

    quote! {
        let mut builder = confers::ConfigBuilder::<Self>::new();

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
            /// Load configuration from all sources.
            ///
            /// This method loads configuration in priority order:
            /// 1. Environment variables (highest priority)
            /// 2. Configuration files
            /// 3. Default values (lowest priority)
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
    // load_file_with_env must carry the same declared-env overrides as
    // load_sync: the generic env source alone cannot honor `name_env`
    // declarations (it only maps UPPER_SNAKE → lower.dot paths).
    let env_calls = generate_env_calls(fields, attrs.effective_env_prefix());
    let skip_assigns = generate_skip_materialization(fields);
    let skip_assigns_in_env_loader = skip_assigns.clone();
    let skip_defaults = generate_skip_default_calls(fields);
    let skip_defaults_in_env_loader = skip_defaults.clone();
    let map_json_calls = generate_map_json_calls(fields);
    let map_json_calls_in_env_loader = map_json_calls.clone();
    let rename_call = generate_rename_all_call(attrs.rename_all.as_ref(), fields);
    let rename_call_in_env_loader = rename_call.clone();
    let mut_kw = if skip_assigns.is_empty() {
        quote! {}
    } else {
        quote! { mut }
    };
    let file_builder_mut = if skip_defaults.is_empty()
        && map_json_calls.is_empty()
        && rename_call.is_none()
    {
        quote! {}
    } else {
        quote! { mut }
    };
    let env_builder_mut = if skip_defaults_in_env_loader.is_empty()
        && env_calls.is_empty()
        && map_json_calls_in_env_loader.is_empty()
        && rename_call_in_env_loader.is_none()
    {
        quote! {}
    } else {
        quote! { mut }
    };

    quote! {
        impl #struct_ident {
            /// Load configuration from a specific file.
            pub fn load_file(path: impl AsRef<std::path::Path>) -> confers::ConfigResult<Self> {
                let #file_builder_mut builder = confers::ConfigBuilder::<Self>::new()
                    .file(path.as_ref());
                #(#skip_defaults)*
                // Field-attribute transforms (rename_all first, then flatten/interpolate)
                #rename_call
                #(#map_json_calls)*
                let #mut_kw config: Self = builder.build()?;
                // `skip` fields are the final word: no source may have set them.
                #(#skip_assigns)*
                Ok(config)
            }

            /// Load configuration from a specific file with environment overrides.
            pub fn load_file_with_env(path: impl AsRef<std::path::Path>) -> confers::ConfigResult<Self> {
                let #env_builder_mut builder = confers::ConfigBuilder::<Self>::new()
                    .file(path.as_ref())
                    .env();
                #(#skip_defaults_in_env_loader)*

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

/// Generate a helper method for getting typed config keys
#[allow(dead_code)]
pub fn generate_typed_keys(
    struct_ident: &Ident,
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> TokenStream {
    let key_defs: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip)
        .map(|(ident, ty, f)| {
            let config_key = f.effective_name();
            let fn_name = format_ident!("key_{}", ident);

            quote! {
                /// Get a typed configuration key for this field.
                pub fn #fn_name() -> confers::TypedConfigKey<#ty> {
                    confers::TypedConfigKey::new(#config_key)
                }
            }
        })
        .collect();

    quote! {
        impl #struct_ident {
            #(#key_defs)*
        }
    }
}
