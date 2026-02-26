//! Procedural macros for the confers configuration library.
//!
//! This crate provides the `#[derive(Config)]` macro for zero-boilerplate
//! configuration loading.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data};
use darling::FromDeriveInput;

mod parse;
mod codegen;

use parse::{StructAttrs, FieldAttrs};
use codegen::{
    generate_defaults_impl, generate_load_impl, generate_validate_impl,
    generate_schema_impl, generate_migration_impl, generate_modules_impl, generate_clap_impl
};
use darling::FromField;

/// Derive macro for configuration loading.
///
/// # Example
///
/// ```ignore
/// use confers::Config;
/// use serde::Deserialize;
///
/// #[derive(Config, Deserialize)]
/// #[config(env_prefix = "APP_")]
/// struct AppConfig {
///     #[config(default = "localhost")]
///     host: String,
///
///     #[config(default = 8080)]
///     port: u16,
///
///     #[config(sensitive = true)]
///     api_key: Option<String>,
/// }
///
/// // Load configuration
/// let config = AppConfig::load().unwrap();
/// ```
///
/// # Struct Attributes
///
/// - `env_prefix = "APP_"` - Prefix for environment variables
/// - `app_name = "myapp"` - Application name for config search
/// - `validate` - Enable validation with garde
/// - `watch` - Enable file watching for hot reload
/// - `version = 1` - Configuration version for migrations
/// - `profile` - Enable APP_ENV profile overlay
///
/// # Field Attributes
///
/// - `default = <expr>` - Default value expression
/// - `description = "..."` - Field description for docs
/// - `name = "key"` - Override configuration key name
/// - `name_env = "VAR"` - Override environment variable name
/// - `sensitive = true` - Mark as sensitive (hidden in logs)
/// - `encrypt = "xchacha20"` - Enable encryption for this field
/// - `flatten` - Flatten nested struct into parent namespace
/// - `skip` - Skip this field during loading
/// - `interpolate = true` - Enable `${VAR:default}` interpolation
/// - `dynamic` - Generate DynamicField handle
/// - `module_group = "group"` - Assign field to a config module group
#[proc_macro_derive(Config, attributes(config))]
pub fn config_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_config_derive(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for generating JSON Schema.
#[proc_macro_derive(ConfigSchema, attributes(config))]
pub fn config_schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_config_schema_derive(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for generating migration support.
#[proc_macro_derive(ConfigMigration, attributes(config))]
pub fn config_migration_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_config_migration_derive(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for generating module registry.
#[proc_macro_derive(ConfigModules, attributes(config))]
pub fn config_modules_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_config_modules_derive(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for generating CLI arguments.
#[proc_macro_derive(ConfigClap, attributes(config))]
pub fn config_clap_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_config_clap_derive(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_config_derive(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // Parse struct-level attributes
    let struct_attrs = StructAttrs::from_derive_input(input)
        .map_err(|e| syn::Error::new_spanned(input, e.to_string()))?;

    // Get the struct identifier
    let struct_ident = &input.ident;

    // Get fields if it's a named struct
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "Config can only be derived for named structs",
            ))
        }
    };

    // Parse field attributes
    let field_info: Vec<(&syn::Ident, &syn::Type, FieldAttrs)> = fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let attrs = FieldAttrs::from_field(field).ok()?;
            Some((ident, &field.ty, attrs))
        })
        .collect();

    // Generate code
    let defaults_impl = generate_defaults_impl(struct_ident, &field_info);
    let load_impl = generate_load_impl(struct_ident, &struct_attrs, fields);
    let validate_impl = generate_validate_impl(&struct_attrs, &field_info);

    Ok(quote! {
        #defaults_impl
        #load_impl
        #validate_impl
    })
}

fn impl_config_schema_derive(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_attrs = StructAttrs::from_derive_input(input)
        .map_err(|e| syn::Error::new_spanned(input, e.to_string()))?;

    let struct_ident = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ConfigSchema can only be derived for named structs",
            ))
        }
    };

    let schema_impl = generate_schema_impl(struct_ident, &struct_attrs, fields);

    Ok(quote! {
        #schema_impl
    })
}

fn impl_config_migration_derive(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_attrs = StructAttrs::from_derive_input(input)
        .map_err(|e| syn::Error::new_spanned(input, e.to_string()))?;

    let struct_ident = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ConfigMigration can only be derived for named structs",
            ))
        }
    };

    let migration_impl = generate_migration_impl(struct_ident, &struct_attrs, fields);

    Ok(quote! {
        #migration_impl
    })
}

fn impl_config_modules_derive(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_attrs = StructAttrs::from_derive_input(input)
        .map_err(|e| syn::Error::new_spanned(input, e.to_string()))?;

    let struct_ident = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ConfigModules can only be derived for named structs",
            ))
        }
    };

    let modules_impl = generate_modules_impl(struct_ident, &struct_attrs, fields);

    Ok(quote! {
        #modules_impl
    })
}

fn impl_config_clap_derive(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_attrs = StructAttrs::from_derive_input(input)
        .map_err(|e| syn::Error::new_spanned(input, e.to_string()))?;

    let struct_ident = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ConfigClap can only be derived for named structs",
            ))
        }
    };

    let clap_impl = generate_clap_impl(struct_ident, &struct_attrs, fields);

    Ok(quote! {
        #clap_impl
    })
}