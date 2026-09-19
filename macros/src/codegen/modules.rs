// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Module registry code generation for Config derive macro.
//!
//! Generates module_registry function for composable config groups.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident};

use crate::parse::{StructAttrs, parse_field_attrs};

/// Generate module registry for config groups.
pub fn generate_modules_impl(
    struct_ident: &Ident,
    _attrs: &StructAttrs,
    fields: &Fields,
) -> TokenStream {
    // Collect unique module group names. Malformed `#[config(...)]`
    // attributes surface as compile errors appended to the output.
    let (field_info, attr_errors) = parse_field_attrs(fields);
    let mut group_names: Vec<TokenStream> = Vec::new();
    let mut seen_groups: Vec<String> = Vec::new();

    for (_, _, attrs) in &field_info {
        if let Some(group) = &attrs.module_group {
            // Avoid duplicates
            if !seen_groups.contains(group) {
                seen_groups.push(group.clone());
                group_names.push(quote! { #group });
            }
        }
    }

    // Generate register_group calls for each module group
    let register_calls: Vec<TokenStream> = group_names
        .iter()
        .map(|group_name| {
            quote! {
                registry.register_group(#group_name, vec![], None);
            }
        })
        .collect();

    quote! {
        #attr_errors
        impl #struct_ident {
            /// Generate a module registry for this configuration type.
            ///
            /// # Example
            ///
            /// ```ignore
            /// let registry = MyConfig::module_registry();
            /// for group in registry.list_groups() {
            ///     println!("Config group: {}", group);
            /// }
            /// ```
            pub fn module_registry() -> confers::modules::ModuleRegistry {
                let mut registry = confers::modules::ModuleRegistry::default();
                #(#register_calls)*
                registry
            }

            /// Get list of all module group names.
            pub fn module_groups() -> Vec<&'static str> {
                vec![#(#group_names),*]
            }
        }
    }
}
