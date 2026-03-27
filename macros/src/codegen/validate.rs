//! Validation code generation for the Config derive macro.
//!
//! When `#[config(validate)]` is set, we don't generate our own Validate impl.
//! Instead, the user should add `#[derive(garde::Validate)]` to their struct
//! and use garde's attributes for validation rules.

use proc_macro2::TokenStream;

use crate::parse::{FieldAttrs, StructAttrs};

/// Generate validation implementation for the struct.
///
/// Note: We don't generate a Validate impl ourselves. Instead, the user should
/// add `#[derive(garde::Validate)]` to their struct. This function exists for
/// future extensions where we might want to generate validation-related code.
pub fn generate_validate_impl(
    struct_attrs: &StructAttrs,
    _fields: &[(&syn::Ident, &syn::Type, FieldAttrs)],
) -> Option<TokenStream> {
    // Only generate if validation is enabled at struct level
    if !struct_attrs.validate {
        return None;
    }

    // We don't generate Validate impl - garde's derive macro does that.
    // In the future, we might generate helper code here for:
    // - Automatic validation in load_sync()
    // - Integration with ConfigBuilder
    // - Validation error transformation

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_empty_struct_no_validation() {
        let attrs = StructAttrs {
            ident: parse_quote!(TestStruct),
            validate: false,
            env_prefix: None,
            app_name: None,
            strict: false,
            watch: false,
            version: None,
            profile: false,
            profile_env: None,
        };

        let result = generate_validate_impl(&attrs, &[]);
        assert!(result.is_none());
    }
}
