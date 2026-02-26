//! Attribute parsing for the Config derive macro.
//!
//! Uses darling for derive-aware attribute parsing with precise error spans.

use darling::{FromDeriveInput, FromField};
use quote::quote;
use syn::{Ident, Type, PathArguments, GenericArgument};

/// Parsed attributes from the struct level.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(config), supports(struct_named))]
#[allow(dead_code)]
pub struct StructAttrs {
    /// The struct identifier
    pub ident: Ident,

    /// Whether to enable validation
    #[darling(default)]
    pub validate: bool,

    /// Environment variable prefix
    pub env_prefix: Option<String>,

    /// Application name for config search
    pub app_name: Option<String>,

    /// Whether to error on unknown CLI arguments
    #[darling(default)]
    pub strict: bool,

    /// Whether to enable file watching
    #[darling(default)]
    pub watch: bool,

    /// Configuration version for migrations
    pub version: Option<u32>,

    /// Whether to enable profile overlay
    #[darling(default)]
    pub profile: bool,

    /// Profile environment variable name
    pub profile_env: Option<String>,
}

impl StructAttrs {
    /// Get the effective environment prefix
    pub fn effective_env_prefix(&self) -> &str {
        self.env_prefix.as_deref().unwrap_or("")
    }

    /// Get the profile environment variable name
    #[allow(dead_code)]
    pub fn effective_profile_env(&self) -> &str {
        self.profile_env.as_deref().unwrap_or("APP_ENV")
    }
}

/// Parsed attributes from a field.
#[derive(Debug, FromField)]
#[darling(attributes(config))]
#[allow(dead_code)]
pub struct FieldAttrs {
    /// Field identifier
    pub ident: Option<Ident>,

    /// Field type
    pub ty: Type,

    /// Default value expression
    pub default: Option<syn::Expr>,

    /// Field description for documentation
    pub description: Option<String>,

    /// Override configuration key name
    pub name: Option<String>,

    /// Override environment variable name
    pub name_env: Option<String>,

    /// CLI long argument name
    pub name_clap_long: Option<String>,

    /// CLI short argument character
    pub name_clap_short: Option<char>,

    /// Whether this field is sensitive (hidden in logs)
    #[darling(default)]
    pub sensitive: bool,

    /// Encryption algorithm for this field
    pub encrypt: Option<String>,

    /// Whether to flatten this field into parent namespace
    #[darling(default)]
    pub flatten: bool,

    /// Whether to skip this field during loading
    #[darling(default)]
    pub skip: bool,

    /// Whether to enable interpolation for this field
    #[darling(default)]
    pub interpolate: bool,

    /// Merge strategy for this field
    pub merge_strategy: Option<String>,

    /// Whether to generate a DynamicField handle
    #[darling(default)]
    pub dynamic: bool,

    /// Module group for this field (config groups)
    pub module_group: Option<String>,
}

impl FieldAttrs {
    /// Get the effective configuration key name
    pub fn effective_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            self.ident
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_default()
        })
    }

    /// Get the effective environment variable name
    pub fn effective_env_name(&self, prefix: &str) -> String {
        if let Some(ref name_env) = self.name_env {
            name_env.clone()
        } else {
            let key = self.effective_name();
            format!("{}{}", prefix, key.to_uppercase().replace('.', "_"))
        }
    }

    /// Check if this field is a SecretString type
    pub fn is_secret_string(&self) -> bool {
        is_secret_type(&self.ty)
    }

    /// Check if this field should be treated as sensitive
    pub fn is_sensitive_effective(&self) -> bool {
        self.sensitive || self.encrypt.is_some() || self.is_secret_string()
    }
}

/// Check if a type is SecretString or SecretBytes
fn is_secret_type(ty: &Type) -> bool {
    let type_str = quote!(#ty).to_string();
    type_str.contains("SecretString") || type_str.contains("SecretBytes")
}

/// Check if a type is Option<T>
pub fn is_option_type(ty: &Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Check if a type is Vec<T>
pub fn is_vec_type(ty: &Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Vec";
        }
    }
    false
}

/// Extract the inner type from Option<T> or Vec<T>
#[allow(dead_code)]
pub fn extract_inner_type(ty: &Type) -> Option<&Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

/// Merge strategy enum for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MergeStrategyKind {
    Replace,
    Join,
    Append,
    Prepend,
    JoinAppend,
    DeepMerge,
}

impl MergeStrategyKind {
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "join" => Self::Join,
            "append" => Self::Append,
            "prepend" => Self::Prepend,
            "join_append" | "joinappend" => Self::JoinAppend,
            "deep_merge" | "deepmerge" => Self::DeepMerge,
            _ => Self::Replace,
        }
    }
}

impl Default for MergeStrategyKind {
    fn default() -> Self {
        Self::Replace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_is_option_type() {
        let ty: Type = parse_quote!(Option<String>);
        assert!(is_option_type(&ty));

        let ty: Type = parse_quote!(String);
        assert!(!is_option_type(&ty));
    }

    #[test]
    fn test_is_vec_type() {
        let ty: Type = parse_quote!(Vec<String>);
        assert!(is_vec_type(&ty));

        let ty: Type = parse_quote!(String);
        assert!(!is_vec_type(&ty));
    }

    #[test]
    fn test_extract_inner_type() {
        let ty: Type = parse_quote!(Option<String>);
        let inner = extract_inner_type(&ty);
        assert!(inner.is_some());

        let ty: Type = parse_quote!(Vec<i32>);
        let inner = extract_inner_type(&ty);
        assert!(inner.is_some());
    }

    #[test]
    fn test_merge_strategy_from_str() {
        assert_eq!(MergeStrategyKind::from_str("replace"), MergeStrategyKind::Replace);
        assert_eq!(MergeStrategyKind::from_str("join"), MergeStrategyKind::Join);
        assert_eq!(MergeStrategyKind::from_str("append"), MergeStrategyKind::Append);
        assert_eq!(MergeStrategyKind::from_str("deep_merge"), MergeStrategyKind::DeepMerge);
    }
}
