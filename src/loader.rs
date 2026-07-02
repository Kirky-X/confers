//! Configuration file loaders with precise error locations.
//!
//! Implementation lives in `crate::impl_::loader`. This module provides
//! the public API surface for configuration file loading.

pub use crate::impl_::loader::{
    check_path_traversal_attempt, detect_format_from_content, detect_format_from_path, load_file,
    normalize_and_validate_path, parse_content, validate_path_with_config, Format, LoaderConfig,
    PathTraversalError,
};

#[cfg(feature = "toml")]
pub use crate::impl_::loader::{parse_toml, parse_toml_table};

#[cfg(feature = "json")]
pub use crate::impl_::loader::{parse_json, parse_json_value};

#[cfg(feature = "yaml")]
pub use crate::impl_::loader::{parse_yaml, parse_yaml_value};

#[cfg(feature = "ini")]
pub use crate::impl_::loader::parse_ini;
