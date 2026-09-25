// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Validation code generation for the Config derive macro.
//!
//! Neither validation flag wires anything into the load pipeline and no
//! `Validate` impl is generated — the rules live in garde's derive
//! (`#[derive(garde::Validate)]` + `#[garde(...)]` field attributes), which
//! the user applies themselves.
//!
//! The two flags differ in what they generate:
//! - `#[config(validate)]` generates **nothing** (historical no-op, kept so
//!   existing code keeps compiling unchanged).
//! - `#[config(validate_helper)]` generates the `confers_validate(&self)`
//!   manual helper: an explicit call site that runs garde validation and
//!   flattens the report into a message string. It requires the
//!   `validation` feature and a derived garde `Validate` impl.

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::{FieldAttrs, StructAttrs};

/// Generate the manual `confers_validate` helper when
/// `#[config(validate_helper)]` is set; `None` (no generated code)
/// otherwise — in particular for the plain `validate` flag, which stays a
/// no-op.
///
/// The helper delegates to garde's `Validate` impl (which the user must
/// derive themselves) through confers' `validator` re-export, so the
/// generated code requires the `validation` feature. It is never invoked
/// automatically — load/builder paths do not call it.
pub fn generate_validate_impl(
    struct_attrs: &StructAttrs,
    _fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> Option<TokenStream> {
    if !struct_attrs.validate_helper {
        return None;
    }

    // The caller splices this stream at top level (outside any `impl`), so
    // the helper brings its own impl block.
    let ident = &struct_attrs.ident;
    Some(quote! {
        impl #ident {
            /// Manual validation helper for `#[config(validate_helper)]`.
            ///
            /// Nothing calls this automatically: the load pipeline and
            /// builder never validate on their own, so the caller decides
            /// when to run it. Delegates to the garde `Validate` impl you
            /// derived (via confers' `validator` re-export; requires the
            /// `validation` feature) and flattens the report into a message
            /// string. For the structured `garde::Report`, call
            /// `garde::Validate::validate` directly.
            pub fn confers_validate(&self) -> ::core::result::Result<(), String> {
                match ::confers::validator::Validate::validate(self) {
                    ::core::result::Result::Ok(()) => ::core::result::Result::Ok(()),
                    ::core::result::Result::Err(report) => {
                        ::core::result::Result::Err(::std::format!("{}", report))
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn attrs(validate: bool, validate_helper: bool) -> StructAttrs {
        StructAttrs {
            ident: parse_quote!(TestStruct),
            validate,
            validate_helper,
            env_prefix: None,
            app_name: None,
            strict: false,
            watch: false,
            version: None,
            rename_all: None,
            profile: false,
            profile_env: None,
        }
    }

    #[test]
    fn test_plain_validate_flag_stays_a_noop() {
        // Historical behavior: `validate` alone generates nothing, so
        // downstream code that only sets the flag keeps compiling.
        let result = generate_validate_impl(&attrs(true, false), &[]);
        assert!(result.is_none());

        let result = generate_validate_impl(&attrs(false, false), &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_helper_attr_generates_manual_helper() {
        let result = generate_validate_impl(&attrs(false, true), &[])
            .expect("validate_helper=true must generate the confers_validate helper");
        let rendered = result.to_string();
        assert!(
            rendered.contains("confers_validate"),
            "generated helper must be named confers_validate, got: {rendered}"
        );
        assert!(
            rendered.contains("validator"),
            "helper must route through confers' validator re-export, got: {rendered}"
        );
    }
}
