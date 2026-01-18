// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use serde::Serialize;
use serde_json::{Number, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
pub use validator::{Validate, ValidationErrors};

/// Trait for configuration validators
pub trait ConfigValidator {
    fn validate(&self) -> Result<(), crate::error::ConfigError>;
}

/// Validate a configuration struct against its validation rules
pub fn validate_struct<T: Validate>(config: &T) -> Result<(), ValidationErrors> {
    config.validate()
}

#[cfg(feature = "schema")]
pub trait SchemaValidatable: Serialize + schemars::JsonSchema {}

#[cfg(feature = "schema")]
impl<T: Serialize + schemars::JsonSchema> SchemaValidatable for T {}

#[cfg(not(feature = "schema"))]
pub trait SchemaValidatable: Serialize {}

#[cfg(not(feature = "schema"))]
impl<T: Serialize> SchemaValidatable for T {}

/// Validate configuration against a JSON Schema (if feature enabled)
#[cfg(feature = "schema")]
pub fn validate_schema<T>(config: &T) -> Result<(), crate::error::ConfigError>
where
    T: Serialize + schemars::JsonSchema,
{
    let schema = schemars::schema_for!(T);
    let schema_json = serde_json::to_value(&schema).map_err(|e| {
        crate::error::ConfigError::FormatDetectionFailed(format!(
            "Schema serialization error: {}",
            e
        ))
    })?;

    let compiled = jsonschema::validator_for(&schema_json).map_err(|e| {
        crate::error::ConfigError::FormatDetectionFailed(format!(
            "Schema compilation error: {:?}",
            e
        ))
    })?;

    let instance = serde_json::to_value(config).map_err(|e| {
        crate::error::ConfigError::FormatDetectionFailed(format!(
            "Config serialization error: {}",
            e
        ))
    })?;

    if let Err(error) = compiled.validate(&instance) {
        return Err(crate::error::ConfigError::FormatDetectionFailed(format!(
            "Schema validation failed: {}",
            error
        )));
    }

    Ok(())
}

/// Parallel validation configuration
#[derive(Debug, Clone, Default)]
pub struct ParallelValidationConfig {
    pub num_threads: Option<usize>,
    pub validate_schema: bool,
    pub validate_struct: bool,
    pub batch_size: usize,
    pub timeout_ms: Option<u64>,
}

impl ParallelValidationConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_num_threads(mut self, threads: usize) -> Self {
        self.num_threads = Some(threads);
        self
    }

    pub fn with_schema_validation(mut self, enabled: bool) -> Self {
        self.validate_schema = enabled;
        self
    }

    pub fn with_struct_validation(mut self, enabled: bool) -> Self {
        self.validate_struct = enabled;
        self
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn num_threads(&self) -> usize {
        self.num_threads.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4)
        })
    }

    pub fn batch_size(&self) -> usize {
        if self.batch_size == 0 {
            100
        } else {
            self.batch_size
        }
    }
}

/// Parallel validation result
#[derive(Debug, Clone)]
pub struct ParallelValidationResult {
    pub struct_errors: Vec<(String, ValidationErrors)>,
    pub schema_errors: Vec<(String, crate::error::ConfigError)>,
    pub success: bool,
}

impl Default for ParallelValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelValidationResult {
    pub fn new() -> Self {
        Self {
            struct_errors: Vec::new(),
            schema_errors: Vec::new(),
            success: true,
        }
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn add_struct_error(&mut self, name: String, errors: ValidationErrors) {
        self.struct_errors.push((name, errors));
        self.success = false;
    }

    pub fn add_schema_error(&mut self, name: String, error: crate::error::ConfigError) {
        self.schema_errors.push((name, error));
        self.success = false;
    }

    pub fn merge(&mut self, other: ParallelValidationResult) {
        self.struct_errors.extend(other.struct_errors);
        self.schema_errors.extend(other.schema_errors);
        if !other.success {
            self.success = false;
        }
    }

    pub fn errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (name, ve) in &self.struct_errors {
            for (field, field_errors) in ve.errors() {
                Self::collect_errors_from_kind(name, field, field_errors, &mut errors);
            }
        }
        for (name, err) in &self.schema_errors {
            errors.push(format!("{}: {}", name, err));
        }
        errors
    }

    fn collect_errors_from_kind(
        name: &str,
        field: &str,
        kind: &validator::ValidationErrorsKind,
        errors: &mut Vec<String>,
    ) {
        match kind {
            validator::ValidationErrorsKind::Field(field_errors) => {
                for error in field_errors.iter() {
                    if let Some(msg) = &error.message {
                        errors.push(format!("{}: {} - {}", name, field, msg));
                    }
                }
            }
            validator::ValidationErrorsKind::Struct(nested) => {
                for (nested_field, nested_errors) in nested.errors() {
                    Self::collect_errors_from_kind(name, nested_field, nested_errors, errors);
                }
            }
            validator::ValidationErrorsKind::List(list) => {
                for (index, list_errors) in list {
                    for (list_field, list_kind) in list_errors.errors() {
                        Self::collect_errors_from_kind(
                            name,
                            &format!("{}[{}]", list_field, index),
                            list_kind,
                            errors,
                        );
                    }
                }
            }
        }
    }
}

/// Parallel configuration validator
#[cfg(feature = "parallel")]
pub struct ParallelValidator {
    config: ParallelValidationConfig,
}

#[cfg(feature = "parallel")]
impl ParallelValidator {
    pub fn new(config: ParallelValidationConfig) -> Self {
        // Limit maximum threads to prevent resource exhaustion
        // Use at most 2x the number of CPU cores
        let max_threads = config.num_threads().min(num_cpus::get() * 2);
        let adjusted_config = ParallelValidationConfig {
            num_threads: Some(max_threads),
            ..config
        };
        Self {
            config: adjusted_config,
        }
    }

    pub fn config(&self) -> &ParallelValidationConfig {
        &self.config
    }

    pub fn validate_many<T, I>(
        &self,
        configs: I,
    ) -> Result<ParallelValidationResult, crate::error::ConfigError>
    where
        T: Validate + SchemaValidatable + Send + Sync + 'static,
        I: IntoIterator<Item = (String, T)>,
    {
        use rayon::prelude::*;

        let config = self.config.clone();
        let mut result = ParallelValidationResult::new();

        let configs_vec: Vec<(String, T)> = configs.into_iter().collect();

        let thread_count = config.num_threads();
        let batch_size = config.batch_size();

        if let Some(timeout_ms) = config.timeout_ms {
            let _timeout = std::time::Duration::from_millis(timeout_ms);
            let handle = std::thread::spawn(move || {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(thread_count)
                    .build()
                    .map_err(|e| crate::error::ConfigError::Other(e.to_string()))?;

                pool.install(|| {
                    let configs_batches: Vec<_> = configs_vec.chunks(batch_size).collect();
                    let mut batch_results = Vec::new();

                    for batch in configs_batches {
                        let batch_result: Vec<ParallelValidationResult> = batch
                            .par_iter()
                            .map(|(name, config_data)| {
                                Self::validate_single(&config, name, config_data)
                            })
                            .collect();
                        batch_results.extend(batch_result);
                    }

                    let mut final_result = ParallelValidationResult::new();
                    for r in batch_results {
                        final_result.merge(r);
                    }
                    Ok::<_, crate::error::ConfigError>(final_result)
                })
            });

            match handle.join() {
                Ok(Ok(res)) => {
                    result.merge(res);
                    Ok(result)
                }
                Ok(Err(e)) => Err(e),
                Err(_) => Err(crate::error::ConfigError::Other(
                    "Validation thread panicked".to_string(),
                )),
            }
        } else {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build()
                .map_err(|e| crate::error::ConfigError::Other(e.to_string()))?;

            pool.install(|| {
                let configs_batches: Vec<_> = configs_vec.chunks(batch_size).collect();
                let mut batch_results = Vec::new();

                for batch in configs_batches {
                    let batch_result: Vec<ParallelValidationResult> = batch
                        .par_iter()
                        .map(|(name, config_data)| {
                            Self::validate_single(&config, name, config_data)
                        })
                        .collect();
                    batch_results.extend(batch_result);
                }

                for r in batch_results {
                    result.merge(r);
                }
                Ok(result)
            })
        }
    }

    fn validate_single<T>(
        config: &ParallelValidationConfig,
        name: &str,
        config_data: &T,
    ) -> ParallelValidationResult
    where
        T: Validate + SchemaValidatable,
    {
        let mut result = ParallelValidationResult::new();

        if config.validate_struct {
            if let Err(e) = config_data.validate() {
                result.add_struct_error(name.to_string(), e);
            }
        }

        if config.validate_schema {
            #[cfg(feature = "schema")]
            {
                if let Err(e) = validate_schema(config_data) {
                    result.add_schema_error(name.to_string(), e);
                }
            }
            #[cfg(not(feature = "schema"))]
            {
                let _ = (config_data, name);
            }
        }

        result
    }

    pub fn validate<T>(&self, name: &str, config: &T) -> Result<(), crate::error::ConfigError>
    where
        T: Validate + SchemaValidatable,
    {
        let result = Self::validate_single(&self.config, name, config);
        if result.is_success() {
            Ok(())
        } else {
            let errors = result.errors();
            if errors.is_empty() {
                Err(crate::error::ConfigError::Other(
                    "Validation failed".to_string(),
                ))
            } else {
                Err(crate::error::ConfigError::Other(errors.join("; ")))
            }
        }
    }
}

#[cfg(not(feature = "parallel"))]
pub struct ParallelValidator;

#[cfg(not(feature = "parallel"))]
impl ParallelValidator {
    pub fn new(_config: ParallelValidationConfig) -> Self {
        Self
    }

    pub fn validate_many<T, I>(
        &self,
        configs: I,
    ) -> Result<ParallelValidationResult, crate::error::ConfigError>
    where
        T: Validate + SchemaValidatable,
        I: IntoIterator<Item = (String, T)>,
    {
        let mut result = ParallelValidationResult::new();
        for (name, config) in configs {
            if config.validate().is_err() {
                result.add_struct_error(name, ValidationErrors::new());
            }
        }
        Ok(result)
    }

    pub fn validate<T>(&self, _name: &str, config: &T) -> Result<(), crate::error::ConfigError>
    where
        T: Validate + SchemaValidatable,
    {
        config.validate().map_err(crate::error::ConfigError::from)
    }
}

// ============================================================================
// Enhanced Configuration Validation System
// ============================================================================

/// Validation error type for categorizing different validation failures
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    RangeError,
    FormatError,
    DependencyError,
    ConsistencyError,
    CustomError,
}

/// Enhanced validation error with detailed information
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field_path: String,
    pub error_type: ValidationErrorType,
    pub message: String,
    pub suggestions: Vec<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Validation error for field '{}': {}",
            self.field_path, self.message
        )
    }
}

impl std::error::Error for ValidationError {}

/// Trait for advanced configuration validators (renamed to avoid conflict)
pub trait AdvancedConfigValidator: Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, config: &Value) -> Result<(), ValidationError>;
    fn priority(&self) -> u8;
}

/// Validation engine that manages and executes multiple validators
pub struct ValidationEngine {
    validators: Vec<Box<dyn AdvancedConfigValidator>>,
}

impl ValidationEngine {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    pub fn add_validator(&mut self, validator: Box<dyn AdvancedConfigValidator>) {
        self.validators.push(validator);
        self.validators.sort_by_key(|v| v.priority());
    }

    /// Validate configuration against all registered validators
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Input Validation**: Always validate user input before use
    /// - ⚠️ **Range Checking**: Ensure numeric values are within expected ranges
    /// - ⚠️ **Error Messages**: Avoid exposing sensitive information in error messages
    /// - ⚠️ **Validation Failures**: Treat validation failures as potential security incidents
    /// - ⚠️ **Validation Logging**: Log all validation failures for security monitoring
    /// - ⚠️ **Validation Bypass**: Never bypass validation for convenience
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::validator::{ValidationEngine, RangeFieldValidator};
    /// # use serde_json::{json, Value};
    /// let mut engine = ValidationEngine::new();
    /// # let config: Value = json!({"port": 8080});
    /// engine.add_validator(Box::new(RangeFieldValidator::new("port", Some(1024.0), Some(65535.0))));
    /// engine.validate(&config)?;
    /// # Ok::<(), Vec<confers::validator::ValidationError>>(())
    /// ```
    pub fn validate(&self, config: &Value) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        for validator in &self.validators {
            if let Err(e) = validator.validate(config) {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Range validator for numeric fields
pub struct RangeFieldValidator {
    field_path: String,
    min: Option<Number>,
    max: Option<Number>,
    inclusive: bool,
}

impl RangeFieldValidator {
    pub fn new(field_path: &str, min: Option<f64>, max: Option<f64>) -> Self {
        Self {
            field_path: field_path.to_string(),
            min: min.map(Number::from_f64).and_then(|n| n),
            max: max.map(Number::from_f64).and_then(|n| n),
            inclusive: true,
        }
    }

    pub fn with_inclusive(mut self, inclusive: bool) -> Self {
        self.inclusive = inclusive;
        self
    }

    fn get_field_value<'a>(&self, config: &'a Value) -> Result<&'a Value, ValidationError> {
        let parts: Vec<&str> = self.field_path.split('.').collect();
        let mut current = config;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part).ok_or_else(|| ValidationError {
                        field_path: self.field_path.clone(),
                        error_type: ValidationErrorType::RangeError,
                        message: format!("Field '{}' not found", part),
                        suggestions: vec![
                            format!("Check if field '{}' exists in configuration", part),
                            "Verify the field path is correct".to_string(),
                        ],
                    })?;
                }
                _ => {
                    return Err(ValidationError {
                        field_path: self.field_path.clone(),
                        error_type: ValidationErrorType::RangeError,
                        message: format!(
                            "Cannot navigate to field '{}' through non-object value",
                            part
                        ),
                        suggestions: vec![
                            "Check the configuration structure".to_string(),
                            "Verify field path is correct".to_string(),
                        ],
                    })
                }
            }
        }

        Ok(current)
    }
}

impl AdvancedConfigValidator for RangeFieldValidator {
    fn name(&self) -> &str {
        "range_validator"
    }

    fn validate(&self, config: &Value) -> Result<(), ValidationError> {
        let value = self.get_field_value(config)?;

        match value {
            Value::Number(num) => {
                if let Some(min) = &self.min {
                    let num_f64 = num.as_f64().unwrap_or(0.0);
                    let min_f64 = min.as_f64().unwrap_or(0.0);

                    if !self.inclusive && num_f64 <= min_f64 {
                        return Err(ValidationError {
                            field_path: self.field_path.clone(),
                            error_type: ValidationErrorType::RangeError,
                            message: format!(
                                "Field '{}' must be greater than {}",
                                self.field_path, min
                            ),
                            suggestions: vec![
                                format!("Increase the value to be greater than {}", min),
                                format!("Use an inclusive validator if {} is acceptable", min),
                            ],
                        });
                    } else if self.inclusive && num_f64 < min_f64 {
                        return Err(ValidationError {
                            field_path: self.field_path.clone(),
                            error_type: ValidationErrorType::RangeError,
                            message: format!(
                                "Field '{}' must be at least {}",
                                self.field_path, min
                            ),
                            suggestions: vec![format!("Increase the value to be at least {}", min)],
                        });
                    }
                }

                if let Some(max) = &self.max {
                    let num_f64 = num.as_f64().unwrap_or(0.0);
                    let max_f64 = max.as_f64().unwrap_or(0.0);

                    if !self.inclusive && num_f64 >= max_f64 {
                        return Err(ValidationError {
                            field_path: self.field_path.clone(),
                            error_type: ValidationErrorType::RangeError,
                            message: format!(
                                "Field '{}' must be less than {}",
                                self.field_path, max
                            ),
                            suggestions: vec![
                                format!("Decrease the value to be less than {}", max),
                                format!("Use an inclusive validator if {} is acceptable", max),
                            ],
                        });
                    } else if self.inclusive && num_f64 > max_f64 {
                        return Err(ValidationError {
                            field_path: self.field_path.clone(),
                            error_type: ValidationErrorType::RangeError,
                            message: format!("Field '{}' must be at most {}", self.field_path, max),
                            suggestions: vec![format!("Decrease the value to be at most {}", max)],
                        });
                    }
                }

                Ok(())
            }
            _ => Err(ValidationError {
                field_path: self.field_path.clone(),
                error_type: ValidationErrorType::RangeError,
                message: format!("Field '{}' is not a number", self.field_path),
                suggestions: vec![
                    "Verify the field type is numeric".to_string(),
                    "Check the configuration file for type errors".to_string(),
                ],
            }),
        }
    }

    fn priority(&self) -> u8 {
        10
    }
}

/// Dependency validator for fields that depend on other fields
#[allow(clippy::type_complexity)]
pub struct DependencyValidator {
    field_path: String,
    depends_on: Vec<String>,
    validator:
        Box<dyn Fn(&Value, &HashMap<String, Value>) -> Result<(), ValidationError> + Send + Sync>,
}

impl DependencyValidator {
    pub fn new<F>(field_path: &str, depends_on: Vec<String>, validator: F) -> Self
    where
        F: Fn(&Value, &HashMap<String, Value>) -> Result<(), ValidationError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            field_path: field_path.to_string(),
            depends_on,
            validator: Box::new(validator),
        }
    }

    fn get_field_value(&self, config: &Value, path: &str) -> Result<Value, ValidationError> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = config;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part).ok_or_else(|| ValidationError {
                        field_path: path.to_string(),
                        error_type: ValidationErrorType::DependencyError,
                        message: format!("Field '{}' not found", part),
                        suggestions: vec![format!("Check if field '{}' exists", part)],
                    })?;
                }
                _ => {
                    return Err(ValidationError {
                        field_path: path.to_string(),
                        error_type: ValidationErrorType::DependencyError,
                        message: format!(
                            "Cannot navigate to field '{}' through non-object value",
                            part
                        ),
                        suggestions: vec!["Check the configuration structure".to_string()],
                    })
                }
            }
        }

        Ok(current.clone())
    }
}

impl AdvancedConfigValidator for DependencyValidator {
    fn name(&self) -> &str {
        "dependency_validator"
    }

    fn validate(&self, config: &Value) -> Result<(), ValidationError> {
        let field_value = self.get_field_value(config, &self.field_path)?;
        let mut dependencies = HashMap::new();

        for dep_path in &self.depends_on {
            let dep_value = self.get_field_value(config, dep_path)?;
            dependencies.insert(dep_path.clone(), dep_value);
        }

        (self.validator)(&field_value, &dependencies)
    }

    fn priority(&self) -> u8 {
        20
    }
}

/// Format validator for string fields with regex patterns
pub struct FormatValidator {
    field_path: String,
    pattern: regex::Regex,
    description: String,
}

impl FormatValidator {
    pub fn new(field_path: &str, pattern: &str, description: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            field_path: field_path.to_string(),
            pattern: regex::Regex::new(pattern)?,
            description: description.to_string(),
        })
    }

    fn get_field_value<'a>(&self, config: &'a Value) -> Result<&'a Value, ValidationError> {
        let parts: Vec<&str> = self.field_path.split('.').collect();
        let mut current = config;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part).ok_or_else(|| ValidationError {
                        field_path: self.field_path.clone(),
                        error_type: ValidationErrorType::FormatError,
                        message: format!("Field '{}' not found", part),
                        suggestions: vec![format!("Check if field '{}' exists", part)],
                    })?;
                }
                _ => {
                    return Err(ValidationError {
                        field_path: self.field_path.clone(),
                        error_type: ValidationErrorType::FormatError,
                        message: format!(
                            "Cannot navigate to field '{}' through non-object value",
                            part
                        ),
                        suggestions: vec!["Check the configuration structure".to_string()],
                    })
                }
            }
        }

        Ok(current)
    }
}

impl AdvancedConfigValidator for FormatValidator {
    fn name(&self) -> &str {
        "format_validator"
    }

    fn validate(&self, config: &Value) -> Result<(), ValidationError> {
        let value = self.get_field_value(config)?;

        match value {
            Value::String(s) => {
                if self.pattern.is_match(s) {
                    Ok(())
                } else {
                    Err(ValidationError {
                        field_path: self.field_path.clone(),
                        error_type: ValidationErrorType::FormatError,
                        message: format!(
                            "Field '{}' does not match required format: {}",
                            self.field_path, self.description
                        ),
                        suggestions: vec![
                            format!("Ensure the value matches the pattern: {}", self.description),
                            "Check for typos or invalid characters".to_string(),
                        ],
                    })
                }
            }
            _ => Err(ValidationError {
                field_path: self.field_path.clone(),
                error_type: ValidationErrorType::FormatError,
                message: format!("Field '{}' is not a string", self.field_path),
                suggestions: vec![
                    "Verify the field type is string".to_string(),
                    "Check the configuration file for type errors".to_string(),
                ],
            }),
        }
    }

    fn priority(&self) -> u8 {
        15
    }
}

/// Consistency validator for checking logical consistency across multiple fields
#[allow(clippy::type_complexity)]
pub struct ConsistencyValidator {
    fields: Vec<String>,
    validator: Box<dyn Fn(&HashMap<String, Value>) -> Result<(), ValidationError> + Send + Sync>,
}

impl ConsistencyValidator {
    pub fn new<F>(fields: Vec<String>, validator: F) -> Self
    where
        F: Fn(&HashMap<String, Value>) -> Result<(), ValidationError> + Send + Sync + 'static,
    {
        Self {
            fields,
            validator: Box::new(validator),
        }
    }

    fn get_field_values(&self, config: &Value) -> Result<HashMap<String, Value>, ValidationError> {
        let mut values = HashMap::new();

        for field_path in &self.fields {
            let parts: Vec<&str> = field_path.split('.').collect();
            let mut current = config;

            for part in parts {
                match current {
                    Value::Object(map) => {
                        current = map.get(part).ok_or_else(|| ValidationError {
                            field_path: field_path.clone(),
                            error_type: ValidationErrorType::ConsistencyError,
                            message: format!("Field '{}' not found", part),
                            suggestions: vec![format!("Check if field '{}' exists", part)],
                        })?;
                    }
                    _ => {
                        return Err(ValidationError {
                            field_path: field_path.clone(),
                            error_type: ValidationErrorType::ConsistencyError,
                            message: format!(
                                "Cannot navigate to field '{}' through non-object value",
                                part
                            ),
                            suggestions: vec!["Check the configuration structure".to_string()],
                        })
                    }
                }
            }

            values.insert(field_path.clone(), current.clone());
        }

        Ok(values)
    }
}

impl AdvancedConfigValidator for ConsistencyValidator {
    fn name(&self) -> &str {
        "consistency_validator"
    }

    fn validate(&self, config: &Value) -> Result<(), ValidationError> {
        let values = self.get_field_values(config)?;
        (self.validator)(&values)
    }

    fn priority(&self) -> u8 {
        25
    }
}

/// Cached validation engine that caches validation results
#[allow(clippy::type_complexity)]
pub struct CachedValidationEngine {
    engine: ValidationEngine,
    cache: Arc<RwLock<HashMap<String, Result<(), Vec<ValidationError>>>>>,
}

impl CachedValidationEngine {
    pub fn new(engine: ValidationEngine) -> Self {
        Self {
            engine,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Validate configuration with caching
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Cache Poisoning**: Ensure cache keys are properly computed and validated
    /// - ⚠️ **Cache Size**: Monitor cache size to prevent memory exhaustion attacks
    /// - ⚠️ **Cache Invalidation**: Invalidate cache when configuration changes
    /// - ⚠️ **Concurrent Access**: Cache is thread-safe but may have race conditions
    /// - ⚠️ **Cache Keys**: Use cryptographic hash for cache keys to prevent collisions
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::validator::{ValidationEngine, CachedValidationEngine};
    /// # use serde_json::{json, Value};
    /// let engine = ValidationEngine::new();
    /// # let config: Value = json!({"port": 8080});
    /// let cached_engine = CachedValidationEngine::new(engine);
    /// cached_engine.validate(&config)?;
    /// # Ok::<(), Vec<confers::validator::ValidationError>>(())
    /// ```
    pub fn validate(&self, config: &Value) -> Result<(), Vec<ValidationError>> {
        let config_hash = self.compute_hash(config);

        // Check cache
        {
            let cache = self.cache.read().unwrap();
            if let Some(result) = cache.get(&config_hash) {
                return result.clone();
            }
        }

        // Execute validation
        let result = self.engine.validate(config);

        // Update cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(config_hash, result.clone());
        }

        result
    }

    fn compute_hash(&self, config: &Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let serialized = serde_json::to_string(config).unwrap_or_else(|_| "{}".to_string());

        let mut hasher = DefaultHasher::new();
        serialized.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_range_validator() {
        let validator = RangeFieldValidator::new("port", Some(1024.0), Some(65535.0));

        let valid_config = json!({ "port": 8080 });
        assert!(validator.validate(&valid_config).is_ok());

        let invalid_config = json!({ "port": 100 });
        assert!(validator.validate(&invalid_config).is_err());
    }

    #[test]
    fn test_format_validator() {
        let validator =
            FormatValidator::new("email", r"^[^@]+@[^@]+\.[^@]+$", "email address").unwrap();

        let valid_config = json!({ "email": "test@example.com" });
        assert!(validator.validate(&valid_config).is_ok());

        let invalid_config = json!({ "email": "invalid" });
        assert!(validator.validate(&invalid_config).is_err());
    }

    #[test]
    fn test_dependency_validator() {
        let validator = DependencyValidator::new(
            "database.url",
            vec![
                "database.username".to_string(),
                "database.password".to_string(),
            ],
            |url, deps| {
                if !url.as_str().unwrap_or("").is_empty()
                    && deps
                        .get("database.username")
                        .unwrap()
                        .as_str()
                        .unwrap_or("")
                        .is_empty()
                {
                    return Err(ValidationError {
                        field_path: "database.url".to_string(),
                        error_type: ValidationErrorType::DependencyError,
                        message: "Database URL requires username".to_string(),
                        suggestions: vec!["Provide database username".to_string()],
                    });
                }
                Ok(())
            },
        );

        let valid_config = json!({
            "database": {
                "url": "postgres://localhost/db",
                "username": "user",
                "password": "pass"
            }
        });
        assert!(validator.validate(&valid_config).is_ok());

        let invalid_config = json!({
            "database": {
                "url": "postgres://localhost/db",
                "username": "",
                "password": "pass"
            }
        });
        assert!(validator.validate(&invalid_config).is_err());
    }

    #[test]
    fn test_consistency_validator() {
        let validator = ConsistencyValidator::new(
            vec!["server.port".to_string(), "server.ssl_port".to_string()],
            |values| {
                let port = values
                    .get("server.port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let ssl_port = values
                    .get("server.ssl_port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                if port == ssl_port {
                    return Err(ValidationError {
                        field_path: "server.port".to_string(),
                        error_type: ValidationErrorType::ConsistencyError,
                        message: "Port and SSL port cannot be the same".to_string(),
                        suggestions: vec!["Use different ports for HTTP and HTTPS".to_string()],
                    });
                }
                Ok(())
            },
        );

        let valid_config = json!({
            "server": {
                "port": 8080,
                "ssl_port": 8443
            }
        });
        assert!(validator.validate(&valid_config).is_ok());

        let invalid_config = json!({
            "server": {
                "port": 8080,
                "ssl_port": 8080
            }
        });
        assert!(validator.validate(&invalid_config).is_err());
    }

    #[test]
    fn test_validation_engine() {
        let mut engine = ValidationEngine::new();

        engine.add_validator(Box::new(RangeFieldValidator::new(
            "port",
            Some(1024.0),
            Some(65535.0),
        )));
        engine.add_validator(Box::new(
            FormatValidator::new("email", r"^[^@]+@[^@]+\.[^@]+$", "email address").unwrap(),
        ));

        let valid_config = json!({
            "port": 8080,
            "email": "test@example.com"
        });
        assert!(engine.validate(&valid_config).is_ok());

        let invalid_config = json!({
            "port": 100,
            "email": "test@example.com"
        });
        assert!(engine.validate(&invalid_config).is_err());
    }

    #[test]
    fn test_cached_validation_engine() {
        let mut engine = ValidationEngine::new();
        engine.add_validator(Box::new(RangeFieldValidator::new(
            "port",
            Some(1024.0),
            Some(65535.0),
        )));

        let cached_engine = CachedValidationEngine::new(engine);

        let config = json!({ "port": 8080 });

        // First validation should execute
        let result1 = cached_engine.validate(&config);
        assert!(result1.is_ok());

        // Second validation should use cache
        let result2 = cached_engine.validate(&config);
        assert!(result2.is_ok());

        // Clear cache and validate again
        cached_engine.clear_cache();
        let result3 = cached_engine.validate(&config);
        assert!(result3.is_ok());
    }
}
