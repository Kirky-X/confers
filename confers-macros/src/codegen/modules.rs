//! Module registry code generation for Config derive macro.
//!
//! Generates module_registry function for composable config groups.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Fields};
use darling::FromField;

use crate::parse::{StructAttrs, FieldAttrs};

/// Generate module registry for config groups.
pub fn generate_modules_impl(
    _struct_ident: &Ident,
    _attrs: &StructAttrs,
    fields: &Fields,
) -> TokenStream {
    // Collect module group fields
    let module_fields: Vec<TokenStream> = fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let attrs = FieldAttrs::from_field(field).ok()?;
            if let Some(group) = &attrs.module_group {
                Some(quote! {
                    #ident => #group
                })
            } else {
                None
            }
        })
        .collect();

    quote! {
        /// Generate a module registry for this configuration type.
        pub fn module_registry() -> confers::modules::ModuleRegistry {
            let mut registry = confers::modules::ModuleRegistry::new();
            #(#module_fields;)*
            registry
        }
    }
}
