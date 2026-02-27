//! Configuration value interpolation.
//!
//! This module provides support for interpolating variable references in string values.
//! Enable the `interpolation` feature to use this module.
//!
//! # Syntax
//!
//! - `${VAR}` - Reference to environment variable `VAR`
//! - `${VAR:default}` - Reference with default value if variable is not set
//! - Nested references are resolved recursively
//!
//! # Sensitive Field Protection
//!
//! When interpolating sensitive fields, additional protections are applied:
//! - Variables referenced from sensitive fields are marked as sensitive
//! - Interpolation audit log tracks which variables were used
//! - Warning is generated when sensitive fields use interpolation
//!
//! # Example
//!
//! # Example
//!
//! ```rust
//! use confers::interpolation::interpolate;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // With HOST=localhost (set env var for test)
//!     std::env::set_var("HOST", "localhost");
//!     let result = interpolate("Server: ${HOST}", &|k| std::env::var(k).ok())?;
//!     assert_eq!(result, "Server: localhost");
//!
//!     // With default value
//!     let result = interpolate("Port: ${PORT:8080}", &|k| std::env::var(k).ok())?;
//!     assert_eq!(result, "Port: 8080");
//!
//!     Ok(())
//! }
//! ```

use std::collections::{HashMap, HashSet};

use crate::error::{ConfigError, ConfigResult};

/// Interpolate variable references in a string.
///
/// # Arguments
///
/// * `template` - The string template containing `${VAR}` or `${VAR:default}` references
/// * `resolver` - A function that resolves variable names to values
///
/// # Returns
///
/// The interpolated string with all references replaced.
///
/// # Errors
///
/// Returns `ConfigError::InterpolationError` if:
/// - A referenced variable is not found and has no default value
/// - A circular reference is detected
pub fn interpolate<F>(template: &str, resolver: &F) -> ConfigResult<String>
where
    F: Fn(&str) -> Option<String>,
{
    let mut visited = HashSet::new();
    interpolate_inner(template, resolver, &mut visited)
}

/// Interpolate with sensitive field tracking.
///
/// This function tracks which variables were referenced during interpolation,
/// useful for audit logging and security analysis.
pub fn interpolate_tracked<F>(
    template: &str,
    resolver: &F,
    is_sensitive: bool,
) -> ConfigResult<InterpolationResult>
where
    F: Fn(&str) -> Option<String>,
{
    let mut visited = HashSet::new();
    let mut referenced_vars = HashSet::new();
    let mut sensitive_refs = HashSet::new();

    let result = interpolate_inner_tracked(
        template,
        resolver,
        &mut visited,
        &mut referenced_vars,
        &mut sensitive_refs,
        is_sensitive,
    )?;

    Ok(InterpolationResult {
        value: result,
        referenced_vars,
        sensitive_refs,
        is_sensitive,
    })
}

/// Result of interpolation with tracking information.
#[derive(Debug, Clone)]
pub struct InterpolationResult {
    /// The interpolated value
    pub value: String,
    /// All variables referenced during interpolation
    pub referenced_vars: HashSet<String>,
    /// Variables that were marked as sensitive
    pub sensitive_refs: HashSet<String>,
    /// Whether the field being interpolated is sensitive
    pub is_sensitive: bool,
}

impl InterpolationResult {
    /// Check if any sensitive variables were referenced.
    pub fn has_sensitive_refs(&self) -> bool {
        !self.sensitive_refs.is_empty()
    }

    /// Get all referenced variable names.
    pub fn referenced_vars(&self) -> impl Iterator<Item = &String> {
        self.referenced_vars.iter()
    }

    /// Check if a specific variable was referenced.
    pub fn referenced(&self, var: &str) -> bool {
        self.referenced_vars.contains(var)
    }
}

/// Inner interpolation function with cycle detection and optional tracking.
///
/// This unified implementation handles both tracked and untracked interpolation
/// to avoid code duplication.
fn interpolate_inner_impl<F>(
    template: &str,
    resolver: &F,
    visited: &mut HashSet<String>,
    referenced_vars: &mut Option<HashSet<String>>,
    sensitive_refs: &mut Option<HashSet<String>>,
    is_sensitive: bool,
) -> ConfigResult<String>
where
    F: Fn(&str) -> Option<String>,
{
    let mut result = String::with_capacity(template.len() * 2);
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        // Check for ${
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'

            // Extract the variable name and optional default
            let mut var_content = String::new();
            let mut depth = 1;

            while depth > 0 {
                match chars.next() {
                    Some('{') => {
                        depth += 1;
                        var_content.push('{');
                    }
                    Some('}') => {
                        depth -= 1;
                        if depth > 0 {
                            var_content.push('}');
                        }
                    }
                    Some(c) => var_content.push(c),
                    None => {
                        return Err(ConfigError::InterpolationError {
                            variable: var_content,
                            message: "unterminated variable reference".to_string(),
                        });
                    }
                }
            }

            // Parse variable name and default value
            let (var_name, default_value) = parse_var_content(&var_content)?;

            // Track referenced variable (if tracking is enabled)
            if let Some(ref_vars) = referenced_vars.as_mut() {
                ref_vars.insert(var_name.to_string());
            }

            // Track sensitive references (if tracking is enabled)
            if is_sensitive {
                if let Some(sens_refs) = sensitive_refs.as_mut() {
                    sens_refs.insert(var_name.to_string());
                }
            }

            // Check for circular reference
            if visited.contains(var_name) {
                return Err(ConfigError::CircularReference {
                    path: var_name.to_string(),
                });
            }

            // Resolve the variable
            let value = if let Some(val) = resolver(var_name) {
                val
            } else if let Some(default) = default_value {
                // Default might contain interpolations too
                visited.insert(var_name.to_string());
                let resolved = interpolate_inner_impl(
                    default,
                    resolver,
                    visited,
                    referenced_vars,
                    sensitive_refs,
                    is_sensitive,
                )?;
                visited.remove(var_name);
                resolved
            } else {
                return Err(ConfigError::InterpolationError {
                    variable: var_name.to_string(),
                    message: "variable not found and no default provided".to_string(),
                });
            };

            // Recursively interpolate the value (it might contain more references)
            visited.insert(var_name.to_string());
            let interpolated = interpolate_inner_impl(
                &value,
                resolver,
                visited,
                referenced_vars,
                sensitive_refs,
                is_sensitive,
            )?;
            visited.remove(var_name);

            result.push_str(&interpolated);
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

/// Inner interpolation function with cycle detection (no tracking).
fn interpolate_inner<F>(
    template: &str,
    resolver: &F,
    visited: &mut HashSet<String>,
) -> ConfigResult<String>
where
    F: Fn(&str) -> Option<String>,
{
    let mut ref_vars = None;
    let mut sens_refs = None;
    interpolate_inner_impl(
        template,
        resolver,
        visited,
        &mut ref_vars,
        &mut sens_refs,
        false,
    )
}

/// Inner interpolation function with cycle detection and tracking.
fn interpolate_inner_tracked<F>(
    template: &str,
    resolver: &F,
    visited: &mut HashSet<String>,
    referenced_vars: &mut HashSet<String>,
    sensitive_refs: &mut HashSet<String>,
    is_sensitive: bool,
) -> ConfigResult<String>
where
    F: Fn(&str) -> Option<String>,
{
    // Take the HashSets and wrap them in Options
    let taken_refs = std::mem::take(referenced_vars);
    let taken_sens = std::mem::take(sensitive_refs);

    let mut ref_vars_opt = Some(taken_refs);
    let mut sens_refs_opt = Some(taken_sens);

    let result = interpolate_inner_impl(
        template,
        resolver,
        visited,
        &mut ref_vars_opt,
        &mut sens_refs_opt,
        is_sensitive,
    )?;

    // Move the values back to the original HashSets
    if let Some(rv) = ref_vars_opt {
        *referenced_vars = rv;
    }
    if let Some(sr) = sens_refs_opt {
        *sensitive_refs = sr;
    }
    Ok(result)
}

/// Parse variable content into (name, default_value).
///
/// Formats:
/// - `VAR` -> (VAR, None)
/// - `VAR:default` -> (VAR, Some(default))
/// - `VAR:-default` -> (VAR, Some(default)) (alternative syntax)
fn parse_var_content(content: &str) -> ConfigResult<(&str, Option<&str>)> {
    let content = content.trim();

    // Handle ${VAR:-default} syntax (common in shell)
    if let Some(pos) = content.find(":-") {
        let name = content[..pos].trim();
        let default = &content[pos + 2..];
        validate_var_name(name)?;
        return Ok((name, Some(default)));
    }

    // Handle ${VAR:default} syntax
    // But be careful not to split on : if it's part of a URL in the default
    // We need to find the first : that's not inside nested ${}
    let mut depth = 0;
    let mut colon_pos = None;

    for (i, c) in content.char_indices() {
        match c {
            '$' if content.as_bytes().get(i + 1) == Some(&b'{') => {
                depth += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
            }
            ':' if depth == 0 && colon_pos.is_none() => {
                colon_pos = Some(i);
            }
            _ => {}
        }
    }

    if let Some(pos) = colon_pos {
        let name = content[..pos].trim();
        let default = &content[pos + 1..];
        validate_var_name(name)?;
        Ok((name, Some(default)))
    } else {
        validate_var_name(content)?;
        Ok((content, None))
    }
}

/// Validate that a variable name is valid.
fn validate_var_name(name: &str) -> ConfigResult<()> {
    if name.is_empty() {
        return Err(ConfigError::InterpolationError {
            variable: "".to_string(),
            message: "empty variable name".to_string(),
        });
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // First character must be letter or underscore
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(ConfigError::InterpolationError {
            variable: name.to_string(),
            message: "variable name must start with letter or underscore".to_string(),
        });
    }

    // Remaining characters must be alphanumeric or underscore
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(ConfigError::InterpolationError {
                variable: name.to_string(),
                message: "variable name can only contain letters, digits, and underscores"
                    .to_string(),
            });
        }
    }

    Ok(())
}

/// Interpolation configuration.
#[derive(Debug, Clone)]
pub struct InterpolationConfig {
    /// Maximum recursion depth for nested interpolations.
    pub max_depth: usize,
    /// Whether to allow unresolved variables (keeps original text).
    pub allow_unresolved: bool,
    /// List of variable names that contain sensitive values.
    pub sensitive_vars: HashSet<String>,
    /// Whether to warn when sensitive fields use interpolation.
    pub warn_sensitive_interpolation: bool,
}

impl Default for InterpolationConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            allow_unresolved: false,
            sensitive_vars: HashSet::new(),
            warn_sensitive_interpolation: true,
        }
    }
}

impl InterpolationConfig {
    /// Create a new interpolation config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a sensitive variable name.
    pub fn with_sensitive_var(mut self, var: impl Into<String>) -> Self {
        self.sensitive_vars.insert(var.into());
        self
    }

    /// Set whether to warn on sensitive interpolation.
    pub fn with_warn_sensitive(mut self, warn: bool) -> Self {
        self.warn_sensitive_interpolation = warn;
        self
    }

    /// Check if a variable is marked as sensitive.
    pub fn is_sensitive(&self, var: &str) -> bool {
        self.sensitive_vars.contains(var)
    }
}

/// Interpolation context for batch interpolation operations.
///
/// Tracks all referenced variables and sensitive references across
/// multiple interpolation operations.
#[derive(Debug, Default)]
pub struct InterpolationContext {
    /// All variables referenced across all interpolations
    all_referenced: HashSet<String>,
    /// Variables referenced from sensitive fields
    sensitive_references: HashMap<String, String>, // var -> field_name
    /// Warnings generated during interpolation
    warnings: Vec<InterpolationWarning>,
}

impl InterpolationContext {
    /// Create a new interpolation context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an interpolation for a field.
    pub fn record(&mut self, field_name: &str, result: &InterpolationResult) {
        // Track all referenced variables
        self.all_referenced
            .extend(result.referenced_vars.iter().cloned());

        // Track sensitive references
        if result.is_sensitive {
            for var in &result.referenced_vars {
                self.sensitive_references
                    .insert(var.clone(), field_name.to_string());
            }
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: InterpolationWarning) {
        self.warnings.push(warning);
    }

    /// Get all warnings.
    pub fn warnings(&self) -> &[InterpolationWarning] {
        &self.warnings
    }

    /// Check if a variable was referenced from a sensitive field.
    pub fn is_sensitive_ref(&self, var: &str) -> bool {
        self.sensitive_references.contains_key(var)
    }

    /// Get the field name that referenced a sensitive variable.
    pub fn sensitive_ref_field(&self, var: &str) -> Option<&str> {
        self.sensitive_references.get(var).map(|s| s.as_str())
    }

    /// Check if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Warning types for interpolation.
#[derive(Debug, Clone)]
pub enum InterpolationWarning {
    /// A sensitive field uses interpolation.
    SensitiveFieldInterpolation {
        /// Field name
        field: String,
        /// Referenced variables
        vars: Vec<String>,
    },
    /// A sensitive variable was referenced.
    SensitiveVarReferenced {
        /// Variable name
        var: String,
        /// Field that referenced it
        field: String,
    },
    /// Circular reference was detected and resolved.
    CircularReferenceResolved {
        /// Variable that caused the cycle
        var: String,
    },
}

impl std::fmt::Display for InterpolationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SensitiveFieldInterpolation { field, vars } => {
                write!(
                    f,
                    "Sensitive field '{}' uses interpolation with variables: {}",
                    field,
                    vars.join(", ")
                )
            }
            Self::SensitiveVarReferenced { var, field } => {
                write!(
                    f,
                    "Sensitive variable '{}' was referenced from field '{}'",
                    var, field
                )
            }
            Self::CircularReferenceResolved { var } => {
                write!(f, "Circular reference resolved for variable '{}'", var)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            vars.iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn test_simple_interpolation() {
        let r = resolver(&[("HOST", "localhost")]);
        let result = interpolate("Server: ${HOST}", &r).unwrap();
        assert_eq!(result, "Server: localhost");
    }

    #[test]
    fn test_default_value() {
        let r = resolver(&[]);
        let result = interpolate("Port: ${PORT:8080}", &r).unwrap();
        assert_eq!(result, "Port: 8080");
    }

    #[test]
    fn test_default_value_with_dash() {
        let r = resolver(&[]);
        let result = interpolate("Port: ${PORT:-8080}", &r).unwrap();
        assert_eq!(result, "Port: 8080");
    }

    #[test]
    fn test_env_overrides_default() {
        let r = resolver(&[("PORT", "443")]);
        let result = interpolate("Port: ${PORT:8080}", &r).unwrap();
        assert_eq!(result, "Port: 443");
    }

    #[test]
    fn test_multiple_references() {
        let r = resolver(&[("HOST", "localhost"), ("PORT", "8080")]);
        let result = interpolate("${HOST}:${PORT}", &r).unwrap();
        assert_eq!(result, "localhost:8080");
    }

    #[test]
    fn test_nested_reference() {
        let r = resolver(&[("HOST", "${DOMAIN}"), ("DOMAIN", "example.com")]);
        let result = interpolate("Server: ${HOST}", &r).unwrap();
        assert_eq!(result, "Server: example.com");
    }

    #[test]
    fn test_circular_reference() {
        let r = resolver(&[("A", "${B}"), ("B", "${A}")]);
        let result = interpolate("${A}", &r);
        assert!(matches!(result, Err(ConfigError::CircularReference { .. })));
    }

    #[test]
    fn test_missing_variable() {
        let r = resolver(&[]);
        let result = interpolate("${UNDEFINED}", &r);
        assert!(matches!(
            result,
            Err(ConfigError::InterpolationError { .. })
        ));
    }

    #[test]
    fn test_missing_variable_with_default() {
        let r = resolver(&[]);
        let result = interpolate("${UNDEFINED:default_value}", &r).unwrap();
        assert_eq!(result, "default_value");
    }

    #[test]
    fn test_empty_default() {
        let r = resolver(&[]);
        let result = interpolate("${VAR:}", &r).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_literal_dollar() {
        let r = resolver(&[]);
        let result = interpolate("Cost: $100", &r).unwrap();
        assert_eq!(result, "Cost: $100");
    }

    #[test]
    fn test_url_in_default() {
        let r = resolver(&[]);
        let result = interpolate("${URL:http://localhost:8080}", &r).unwrap();
        assert_eq!(result, "http://localhost:8080");
    }

    #[test]
    fn test_nested_in_default() {
        let r = resolver(&[("BASE", "localhost")]);
        let result = interpolate("${URL:http://${BASE}:8080}", &r).unwrap();
        assert_eq!(result, "http://localhost:8080");
    }

    #[test]
    fn test_invalid_var_name() {
        let r = resolver(&[]);
        let result = interpolate("${123invalid}", &r);
        assert!(matches!(
            result,
            Err(ConfigError::InterpolationError { .. })
        ));
    }

    #[test]
    fn test_unterminated_reference() {
        let r = resolver(&[]);
        let result = interpolate("${VAR", &r);
        assert!(matches!(
            result,
            Err(ConfigError::InterpolationError { .. })
        ));
    }

    // Tests for sensitive field protection

    #[test]
    fn test_interpolate_tracked() {
        let r = resolver(&[("HOST", "localhost"), ("PORT", "8080")]);
        let result = interpolate_tracked("${HOST}:${PORT}", &r, false).unwrap();
        assert_eq!(result.value, "localhost:8080");
        assert!(result.referenced("HOST"));
        assert!(result.referenced("PORT"));
        assert!(!result.is_sensitive);
    }

    #[test]
    fn test_sensitive_interpolation() {
        let r = resolver(&[("API_KEY", "secret123")]);
        let result = interpolate_tracked("${API_KEY}", &r, true).unwrap();
        assert_eq!(result.value, "secret123");
        assert!(result.referenced("API_KEY"));
        assert!(result.has_sensitive_refs());
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_interpolation_context() {
        let r = resolver(&[("HOST", "localhost"), ("API_KEY", "secret")]);
        let mut ctx = InterpolationContext::new();

        // Record normal field
        let result = interpolate_tracked("${HOST}", &r, false).unwrap();
        ctx.record("server_host", &result);

        // Record sensitive field
        let result = interpolate_tracked("${API_KEY}", &r, true).unwrap();
        ctx.record("api_key", &result);

        // Check tracking
        assert!(ctx.is_sensitive_ref("API_KEY"));
        assert!(!ctx.is_sensitive_ref("HOST"));
        assert_eq!(ctx.sensitive_ref_field("API_KEY"), Some("api_key"));
    }

    #[test]
    fn test_interpolation_warning() {
        let warning = InterpolationWarning::SensitiveFieldInterpolation {
            field: "password".to_string(),
            vars: vec!["DB_PASSWORD".to_string()],
        };
        assert!(warning.to_string().contains("password"));
        assert!(warning.to_string().contains("DB_PASSWORD"));
    }

    #[test]
    fn test_config_sensitive_vars() {
        let config = InterpolationConfig::new()
            .with_sensitive_var("API_KEY")
            .with_sensitive_var("DB_PASSWORD");

        assert!(config.is_sensitive("API_KEY"));
        assert!(config.is_sensitive("DB_PASSWORD"));
        assert!(!config.is_sensitive("HOST"));
    }
}
