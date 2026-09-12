// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Default value generation for Config derive macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

use crate::parse::FieldAttrs;

/// Generate default implementation for a struct.
pub fn generate_defaults_impl(
    struct_ident: &Ident,
    fields: &[(&Ident, &Type, FieldAttrs)],
) -> TokenStream {
    let field_inits: Vec<TokenStream> = fields
        .iter()
        .map(|(ident, ty, attrs)| {
            if let Some(ref default_expr) = attrs.default {
                // Use provided default expression
                quote! {
                    #ident: #default_expr
                }
            } else if crate::parse::is_option_type(ty) {
                // Option<T> defaults to None
                quote! {
                    #ident: None
                }
            } else if crate::parse::is_vec_type(ty) {
                // Vec<T> defaults to empty
                quote! {
                    #ident: Vec::new()
                }
            } else {
                // Try Default::default()
                quote! {
                    #ident: Default::default()
                }
            }
        })
        .collect();

    quote! {
        impl Default for #struct_ident {
            fn default() -> Self {
                Self {
                    #(#field_inits),*
                }
            }
        }
    }
}
