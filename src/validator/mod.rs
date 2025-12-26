// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use serde::Serialize;
pub use validator::{Validate, ValidationErrors};

/// Configuration validator trait
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
        Self { config }
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
