//! Migration code generation for Config derive macro.
//!
//! Generates Versioned trait implementation and migration_registry function.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Fields};
use darling::FromField;

use crate::parse::{StructAttrs, FieldAttrs};

/// Generate Versioned implementation and migration registry for a config struct.
pub fn generate_migration_impl(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &Fields,
) -> TokenStream {
    let version = attrs.version.unwrap_or(1);

    // Collect fields with their migration info
    let _field_migrations: Vec<TokenStream> = fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let field_attrs = FieldAttrs::from_field(field).ok()?;
            if field_attrs.skip {
                return None;
            }
            Some(quote! {
                field: #ident
            })
        })
        .collect();

    quote! {
        impl confers::migration::Versioned for #struct_ident {
            const VERSION: u32 = #version;
        }

        /// Generate a migration registry for this configuration type.
        pub fn migration_registry() -> confers::migration::MigrationRegistry {
            confers::migration::MigrationRegistry::new()
        }
    }
}
