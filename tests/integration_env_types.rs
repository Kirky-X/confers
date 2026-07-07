// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! End-to-end regression test for EnvSource type inference (fix-0.4.1 Bug 2).
//!
//! Prior to fix-0.4.1, `EnvSource::collect()` returned all values as
//! `ConfigValue::String`, which broke `serde::Deserialize` for structs with
//! non-string fields (e.g. `u32`, `bool`). This test loads a typed config
//! purely from environment variables and verifies that `port` (u32) and
//! `debug` (bool) deserialize correctly.

mod common;

use serde::Deserialize;
use serial_test::serial;

use confers::ConfigBuilder;

#[derive(Debug, Default, PartialEq, Deserialize)]
struct TypedConfig {
    port: u32,
    debug: bool,
    host: String,
}

/// Unique prefix to avoid collisions with real env vars in the test runner.
const PREFIX: &str = "TYPEDCFG_";

fn set_test_env() {
    std::env::set_var(format!("{PREFIX}PORT"), "8080");
    std::env::set_var(format!("{PREFIX}DEBUG"), "true");
    std::env::set_var(format!("{PREFIX}HOST"), "localhost");
}

fn cleanup_test_env() {
    std::env::remove_var(format!("{PREFIX}PORT"));
    std::env::remove_var(format!("{PREFIX}DEBUG"));
    std::env::remove_var(format!("{PREFIX}HOST"));
}

#[test]
#[serial]
fn test_env_vars_deserialize_into_typed_struct() {
    set_test_env();

    // Ensure cleanup runs even if the test panics
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let config: TypedConfig = ConfigBuilder::new()
            .env_prefix(PREFIX)
            .build()
            .expect("env vars with TYPEDCFG_ prefix should deserialize into TypedConfig");

        assert_eq!(
            config.port, 8080,
            "u32 field 'port' should deserialize from string '8080'"
        );
        assert_eq!(
            config.debug, true,
            "bool field 'debug' should deserialize from string 'true'"
        );
        assert_eq!(
            config.host, "localhost",
            "string field 'host' should deserialize from 'localhost'"
        );
    }));

    cleanup_test_env();

    match result {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
