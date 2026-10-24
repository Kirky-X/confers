// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Stable `error_type` vocabulary for the config-bus transports.

use crate::error::sanitize::sanitize_error_message;

/// Build the `error_type` of a transport failure.
///
/// The label is a fixed vocabulary entry (stable across releases and
/// locales); the transport error is appended only after sanitization, so no
/// credential, endpoint or path carried by the driver can reach the error's
/// `Debug` rendering.
pub(crate) fn transport_error_type(label: &str, err: &impl std::fmt::Display) -> String {
    format!("{label}: {}", sanitize_error_message(&err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_is_stable_and_detail_is_sanitized() {
        let err = std::io::Error::other("cannot connect to redis://user:hunter2@cache:6379/");
        let error_type = transport_error_type("redis_connection_failed", &err);
        assert!(
            error_type.starts_with("redis_connection_failed: "),
            "got: {error_type}"
        );
        assert!(!error_type.contains("hunter2"), "got: {error_type}");
    }
}
