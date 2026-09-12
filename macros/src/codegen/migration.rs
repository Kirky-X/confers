// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Migration code generation for Config derive macro.
//!
//! Generates Versioned trait implementation and migration_registry function.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident};

use crate::parse::{StructAttrs, parse_field_attrs};

/// Generate Versioned implementation and migration registry for a config struct.
pub fn generate_migration_impl(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &Fields,
) -> TokenStream {
    let version = attrs.version.unwrap_or(1);

    // Malformed `#[config(...)]` attributes surface as compile errors
    // appended to the output instead of dropping the field.
    let (field_info, attr_errors) = parse_field_attrs(fields);

    // Collect fields with their migration info
    let _field_migrations: Vec<TokenStream> = field_info
        .iter()
        .filter(|(_, _, field_attrs)| !field_attrs.skip)
        .map(|(ident, _, _)| {
            quote! {
                field: #ident
            }
        })
        .collect();

    quote! {
        #attr_errors
        impl confers::migration::Versioned for #struct_ident {
            const VERSION: u32 = #version;
        }

        /// Generate a migration registry for this configuration type.
        pub fn migration_registry() -> confers::migration::MigrationRegistry {
            confers::migration::MigrationRegistry::new()
        }
    }
}
