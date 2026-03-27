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

/// Generate the async load() method
fn generate_load_method(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> TokenStream {
    let env_prefix = attrs.effective_env_prefix();

    // Generate default source setup
    let default_calls: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip && f.default.is_some())
        .map(|(_, _, f)| {
            let config_key = f.effective_name();
            let default_expr = f.default.as_ref().unwrap();

            quote! {
                builder = builder.default(#config_key.to_string(), {
                    let val: confers::ConfigValue = (#default_expr).into();
                    val
                });
            }
        })
        .collect();

    // Generate env source setup
    let env_calls: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip)
        .map(|(_, _, f)| {
            let env_name = f.effective_env_name(env_prefix);
            let config_key = f.effective_name();

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
                                    env_map.insert(#config_key.to_string(), confers::ConfigValue::string(val));
                                }
                            }
                            Err(_) => {
                                // Silently skip invalid secret file paths
                            }
                        }
                    } else if let Ok(val) = std::env::var(#env_name) {
                        env_map.insert(#config_key.to_string(), confers::ConfigValue::string(val));
                    }
                }
            } else {
                quote! {
                    if let Ok(val) = std::env::var(#env_name) {
                        env_map.insert(#config_key.to_string(), confers::ConfigValue::string(val));
                    }
                }
            }
        })
        .collect();

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
                let mut builder = confers::ConfigBuilder::<Self>::new();

                // Add defaults first (lowest priority)
                #(#default_calls)*

                // Add environment variables (higher priority)
                let mut env_map = std::collections::HashMap::new();
                #(#env_calls)*
                if !env_map.is_empty() {
                    builder = builder.memory(env_map);
                }

                builder.build()
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
    let env_prefix = attrs.effective_env_prefix();

    // Generate default source setup
    let default_calls: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip && f.default.is_some())
        .map(|(_ident, _, f)| {
            let config_key = f.effective_name();
            let default_expr = f.default.as_ref().unwrap();

            quote! {
                builder = builder.default(#config_key.to_string(), {
                    let val: confers::ConfigValue = (#default_expr).into();
                    val
                });
            }
        })
        .collect();

    // Generate env source setup
    let env_calls: Vec<TokenStream> = fields
        .iter()
        .filter(|(_, _, f)| !f.skip)
        .map(|(_ident, _ty, f)| {
            let env_name = f.effective_env_name(env_prefix);
            let config_key = f.effective_name();

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
                                    env_map.insert(#config_key.to_string(), confers::ConfigValue::string(val));
                                }
                            }
                            Err(_) => {
                                // Silently skip invalid secret file paths
                            }
                        }
                    } else if let Ok(val) = std::env::var(#env_name) {
                        env_map.insert(#config_key.to_string(), confers::ConfigValue::string(val));
                    }
                }
            } else {
                quote! {
                    if let Ok(val) = std::env::var(#env_name) {
                        env_map.insert(#config_key.to_string(), confers::ConfigValue::string(val));
                    }
                }
            }
        })
        .collect();

    quote! {
        impl #struct_ident {
            /// Build configuration with environment variables and defaults.
            pub fn build_config() -> confers::ConfigResult<Self> {
                let mut builder = confers::ConfigBuilder::<Self>::new();

                // Add defaults first (lowest priority)
                #(#default_calls)*

                // Add environment variables (higher priority)
                let mut env_map = std::collections::HashMap::new();
                #(#env_calls)*
                if !env_map.is_empty() {
                    builder = builder.memory(env_map);
                }

                builder.build()
            }
        }
    }
}

/// Generate the load_file() method
fn generate_load_file_method(
    struct_ident: &Ident,
    _attrs: &StructAttrs,
    _fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> TokenStream {
    quote! {
        impl #struct_ident {
            /// Load configuration from a specific file.
            pub fn load_file(path: impl AsRef<std::path::Path>) -> confers::ConfigResult<Self> {
                let builder = confers::ConfigBuilder::<Self>::new()
                    .file(path.as_ref());
                builder.build()
            }

            /// Load configuration from a specific file with environment overrides.
            pub fn load_file_with_env(path: impl AsRef<std::path::Path>) -> confers::ConfigResult<Self> {
                let builder = confers::ConfigBuilder::<Self>::new()
                    .file(path.as_ref())
                    .env();
                builder.build()
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
