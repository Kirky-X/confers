//! Schema generation for Config derive macro.
//!
//! Generates JSON Schema and TypeScript type definitions from configuration structs.

use darling::FromField;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, Type};

use crate::parse::{FieldAttrs, StructAttrs};

/// Generate JSON Schema for a configuration struct.
pub fn generate_schema_impl(
    struct_ident: &Ident,
    _attrs: &StructAttrs,
    fields: &Fields,
) -> TokenStream {
    let field_schemas = generate_field_schemas(fields);

    quote! {
        impl #struct_ident {
            /// Generate JSON Schema for this configuration struct.
            pub fn json_schema() -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "title": stringify!(#struct_ident),
                    "properties": { #field_schemas }
                })
            }

            /// Generate TypeScript type definition for this configuration struct.
            pub fn typescript_type() -> String {
                format!(
                    "export interface {} {{ /* fields */ }}",
                    stringify!(#struct_ident)
                )
            }
        }
    }
}

/// Generate schema for each field.
fn generate_field_schemas(fields: &Fields) -> TokenStream {
    let field_defs: Vec<TokenStream> = fields
        .iter()
        .filter_map(|field| {
            let _ident = field.ident.as_ref()?;
            let attrs = FieldAttrs::from_field(field).ok()?;
            if attrs.skip {
                return None;
            }

            let field_name = attrs.effective_name();
            let field_type = &field.ty;
            let schema = generate_type_schema(field_type);

            Some(quote! {
                #field_name: #schema
            })
        })
        .collect();

    quote! { #(#field_defs),* }
}

/// Generate JSON Schema for a Rust type.
fn generate_type_schema(ty: &Type) -> TokenStream {
    let type_str = quote!(#ty).to_string();

    // Handle common types
    if type_str.contains("String") || type_str.contains("str") {
        return quote! { { "type": "string" } };
    }
    if type_str.contains("i8")
        || type_str.contains("i16")
        || type_str.contains("i32")
        || type_str.contains("i64")
        || type_str.contains("isize")
    {
        return quote! { { "type": "integer" } };
    }
    if type_str.contains("u8")
        || type_str.contains("u16")
        || type_str.contains("u32")
        || type_str.contains("u64")
        || type_str.contains("usize")
    {
        return quote! { { "type": "integer", "minimum": 0 } };
    }
    if type_str.contains("f32") || type_str.contains("f64") {
        return quote! { { "type": "number" } };
    }
    if type_str.contains("bool") {
        return quote! { { "type": "boolean" } };
    }
    if type_str.contains("Vec") || type_str.contains("Array") {
        return quote! { { "type": "array" } };
    }
    if type_str.contains("HashMap") || type_str.contains("Map") || type_str.contains("BTreeMap") {
        return quote! { { "type": "object" } };
    }
    if type_str.contains("Option") {
        return quote! { { "type": ["string", "null"] } };
    }

    // Default to string for unknown types
    quote! { { "type": "string" } }
}
