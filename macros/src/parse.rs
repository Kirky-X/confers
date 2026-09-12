// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Attribute parsing for the Config derive macro.
//!
//! Uses darling for derive-aware attribute parsing with precise error spans.

use darling::{FromDeriveInput, FromField};
use syn::{Ident, Type};

/// Maximum allowed length for environment variable prefix.
const MAX_PREFIX_LENGTH: usize = 64;

/// Maximum allowed length for names (app_name, etc.).
const MAX_NAME_LENGTH: usize = 256;

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

    /// Batch naming strategy for the struct's configuration keys
    /// (`camelCase`, `snake_case`, `kebab-case`). Configuration files are
    /// then addressed with the renamed keys; the derive codegen maps them
    /// back to the serde field names before deserialization.
    #[darling(default)]
    pub rename_all: Option<String>,

    /// Whether to enable profile overlay
    #[darling(default)]
    pub profile: bool,

    /// Profile environment variable name
    pub profile_env: Option<String>,
}

impl StructAttrs {
    /// Get the effective environment prefix.
    pub fn effective_env_prefix(&self) -> &str {
        self.env_prefix.as_deref().unwrap_or("")
    }

    /// Validate struct attributes.
    ///
    /// This method performs comprehensive validation of all struct-level attributes:
    /// - Version must be positive (if specified)
    /// - env_prefix must not be empty, must not exceed max length, and must only contain
    ///   alphanumeric characters and underscores
    /// - app_name must not be empty and must not exceed max length
    ///
    /// # Arguments
    ///
    /// * `input` - The derive input for error span reporting
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all validations pass, or accumulates errors.
    pub fn validate(&self, input: &syn::DeriveInput) -> darling::Result<()> {
        let mut errors = darling::Error::accumulator();

        // Validate rename_all style
        if let Some(ref style) = self.rename_all {
            match style.as_str() {
                "camelCase" | "snake_case" | "kebab-case" => {}
                other => {
                    errors.push(
                        darling::Error::custom(format!(
                            "unsupported rename_all style '{other}'\n\
                             supported styles: \"camelCase\", \"snake_case\", \"kebab-case\""
                        ))
                        .with_span(&input.ident),
                    );
                }
            }
        }

        // Validate version
        if let Some(version) = self.version
            && version == 0
        {
            errors.push(
                darling::Error::custom("version must be a positive integer (1 or greater)")
                    .with_span(&input.ident),
            );
        }

        // Validate env_prefix
        if let Some(ref prefix) = self.env_prefix {
            // Length check
            if prefix.len() > MAX_PREFIX_LENGTH {
                errors.push(
                    darling::Error::custom(format!(
                        "env_prefix exceeds maximum length of {} characters (current: {})",
                        MAX_PREFIX_LENGTH,
                        prefix.len()
                    ))
                    .with_span(&input.ident),
                );
            }

            // Empty check
            if prefix.is_empty() {
                errors.push(
                    darling::Error::custom(
                        "env_prefix cannot be empty. Remove the attribute to use no prefix",
                    )
                    .with_span(&input.ident),
                );
            }

            // Character whitelist check
            if !prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                errors.push(
                    darling::Error::custom(
                        "env_prefix must only contain alphanumeric characters and underscores",
                    )
                    .with_span(&input.ident),
                );
            }

            // Control character check
            if prefix.chars().any(|c| c.is_control()) {
                errors.push(
                    darling::Error::custom("env_prefix cannot contain control characters")
                        .with_span(&input.ident),
                );
            }
        }

        // Validate app_name
        if let Some(ref app_name) = self.app_name {
            if app_name.len() > MAX_NAME_LENGTH {
                errors.push(
                    darling::Error::custom(format!(
                        "app_name exceeds maximum length of {} characters",
                        MAX_NAME_LENGTH
                    ))
                    .with_span(&input.ident),
                );
            }

            if app_name.is_empty() {
                errors.push(
                    darling::Error::custom("app_name cannot be empty").with_span(&input.ident),
                );
            }
        }

        errors.finish()
    }
}

/// Parsed attributes from a field.
#[derive(Debug, FromField)]
#[darling(attributes(config))]
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
    ///
    /// Requires the `security` feature on the confers dependency: the
    /// generated `_FILE` env handling references
    /// `confers::security::PathValidator`.
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

    /// Whether to subscribe this field in the generated field-level
    /// hot-reload watcher (`field_watcher`)
    #[darling(default)]
    pub watch: bool,

    /// Module group for this field (config groups)
    pub module_group: Option<String>,

    /// `#[serde(rename = "...")]` / `#[serde(rename(deserialize = "..."))]`
    /// value captured from the field's serde attributes (`darling(skip)`:
    /// not part of `#[config]`, filled by [`FieldAttrs::from_field_with_serde`]).
    #[darling(skip)]
    pub serde_rename: Option<String>,
}

impl FieldAttrs {
    /// Parse the `#[config(...)]` attributes plus the serde rename attribute.
    ///
    /// Prefer this over [`FromField::from_field`]: generated keys must follow
    /// the serde field name (the merge-space key), which differs from the
    /// Rust identifier whenever `#[serde(rename)]` is present.
    pub fn from_field_with_serde(field: &syn::Field) -> darling::Result<Self> {
        let mut attrs = Self::from_field(field)?;
        attrs.serde_rename = serde_rename_attr(field);
        Ok(attrs)
    }

    /// Get the effective configuration key name
    pub fn effective_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            self.ident
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_default()
        })
    }

    /// The serde field name: `#[serde(rename)]` wins over the Rust
    /// identifier. This is the merge-space key used for defaults, env
    /// overrides, flatten hoisting, and the field-watcher extractors.
    pub fn serde_name(&self) -> String {
        self.serde_rename.clone().unwrap_or_else(|| {
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

    /// Validate field attributes and return errors with helpful suggestions
    pub fn validate(&self, _field: &syn::Field) -> darling::Result<()> {
        let mut errors = darling::Error::accumulator();

        // Validate encrypt algorithm
        if let Some(ref algo) = self.encrypt {
            match algo.as_str() {
                "xchacha20" | "aes256-gcm" => {}
                _ => {
                    if let Some(ident) = self.ident.as_ref() {
                        errors.push(
                            darling::Error::custom(format!(
                                "unsupported encryption algorithm '{}'\n\
                                 supported algorithms: \"xchacha20\", \"aes256-gcm\"",
                                algo
                            ))
                            .with_span(ident),
                        );
                    }
                }
            }
        }

        // Validate merge_strategy
        if let Some(ref strategy) = self.merge_strategy {
            let valid_strategies = ["replace", "join", "append", "prepend", "join_append"];
            if !valid_strategies.contains(&strategy.as_str())
                && let Some(ident) = self.ident.as_ref()
            {
                errors.push(
                    darling::Error::custom(format!(
                        "invalid merge strategy '{}'\n\
                         valid strategies: {}",
                        strategy,
                        valid_strategies.join(", ")
                    ))
                    .with_span(ident),
                );
            }
        }

        // Validate sensitive field type
        if self.sensitive
            && !self.is_secret_string()
            && let Some(ident) = self.ident.as_ref()
        {
            errors.push(
                darling::Error::custom(format!(
                    "sensitive field '{}' should use SecretString or SecretBytes type for security",
                    ident
                ))
                .with_span(ident),
            );
        }

        errors.finish()
    }
}

/// Check if a type is SecretString or SecretBytes (optimized version)
pub fn is_secret_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "SecretString" || segment.ident == "SecretBytes";
    }
    false
}

/// Check if a type is Option<T>
pub fn is_option_type(ty: &Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

/// Check if a type is Vec<T>
pub fn is_vec_type(ty: &Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Vec";
    }
    false
}

/// Extract the serde field rename from a field's attributes.
///
/// Handles both `#[serde(rename = "x")]` and
/// `#[serde(rename(serialize = "a", deserialize = "b"))]` — for the merge
/// space only the deserialization name matters.
fn serde_rename_attr(field: &syn::Field) -> Option<String> {
    for attr in &field.attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let Ok(nested) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };
        for meta in nested {
            if !meta.path().is_ident("rename") {
                continue;
            }
            match meta {
                syn::Meta::NameValue(nv) => {
                    if let Some(v) = expr_as_string_lit(&nv.value) {
                        return Some(v);
                    }
                }
                syn::Meta::List(list) => {
                    let Ok(inner) = list.parse_args_with(
                        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                    ) else {
                        continue;
                    };
                    for m in inner {
                        if m.path().is_ident("deserialize")
                            && let syn::Meta::NameValue(nv) = m
                            && let Some(v) = expr_as_string_lit(&nv.value)
                        {
                            return Some(v);
                        }
                    }
                }
                syn::Meta::Path(_) => {}
            }
        }
    }
    None
}

fn expr_as_string_lit(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(lit) = expr
        && let syn::Lit::Str(s) = &lit.lit
    {
        return Some(s.value());
    }
    None
}

/// Parse the `#[config(...)]` attributes of every field of a named struct.
///
/// Malformed attributes are never silently dropped: the collected `darling`
/// errors come back as a token stream of spanned `compile_error!` diagnostics
/// that the caller must append to its generated output. Returns an empty vec
/// when `fields` is not a named-field struct (the callers then emit nothing).
pub fn parse_field_attrs(
    fields: &syn::Fields,
) -> (
    Vec<(&syn::Ident, &syn::Type, FieldAttrs)>,
    proc_macro2::TokenStream,
) {
    let syn::Fields::Named(named) = fields else {
        return (Vec::new(), proc_macro2::TokenStream::new());
    };
    let mut errors = darling::Error::accumulator();
    let info: Vec<(&syn::Ident, &syn::Type, FieldAttrs)> = named
        .named
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            match FieldAttrs::from_field_with_serde(field) {
                Ok(attrs) => Some((ident, &field.ty, attrs)),
                Err(e) => {
                    errors.push(e);
                    None
                }
            }
        })
        .collect();
    let error_tokens = errors
        .finish()
        .err()
        .map(|e| e.write_errors())
        .unwrap_or_default();
    (info, error_tokens)
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
    fn test_serde_rename_capture() {
        let field: syn::Field = parse_quote! {
            #[serde(rename = "hostName")]
            #[config(default = "h")]
            pub host: String
        };
        let attrs = FieldAttrs::from_field_with_serde(&field).unwrap();
        assert_eq!(attrs.serde_name(), "hostName");

        let field: syn::Field = parse_quote! {
            #[serde(rename(serialize = "a", deserialize = "b"))]
            pub host: String
        };
        let attrs = FieldAttrs::from_field_with_serde(&field).unwrap();
        assert_eq!(attrs.serde_name(), "b", "deserialize name wins");

        let field: syn::Field = parse_quote! {
            #[serde(rename_all = "camelCase")]
            pub host_name: String
        };
        let attrs = FieldAttrs::from_field_with_serde(&field).unwrap();
        assert_eq!(
            attrs.serde_name(),
            "host_name",
            "field-level rename_all is not the merge key form"
        );
    }

    #[test]
    fn test_parse_field_attrs_collects_errors() {
        let input: syn::DeriveInput = parse_quote! {
            struct Sample {
                #[config(default = "ok")]
                pub fine: String,
                #[config(encrypt = 42)]
                pub broken: String
            }
        };
        let fields = match &input.data {
            syn::Data::Struct(data) => &data.fields,
            _ => unreachable!("parsed a struct"),
        };
        let (info, errors) = parse_field_attrs(fields);
        assert_eq!(info.len(), 1, "the well-formed field still parses");
        assert!(
            !errors.is_empty(),
            "the malformed attribute must surface as compile-error tokens"
        );
    }
}
