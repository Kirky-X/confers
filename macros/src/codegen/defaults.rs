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

/// Generate a function to check if a field has an explicit default
#[allow(dead_code)]
pub fn has_explicit_default(attrs: &FieldAttrs) -> bool {
    attrs.default.is_some()
}

/// Generate the default value expression for a field
#[allow(dead_code)]
pub fn default_value_expr(attrs: &FieldAttrs) -> Option<TokenStream> {
    attrs.default.as_ref().map(|expr| quote!(#expr))
}
