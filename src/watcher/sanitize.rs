// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Reason/message sanitizing shared by the watcher components.

/// Flatten a validator/check/error reason onto a single bounded line.
///
/// Reasons flow verbatim into logs and the canary change stream, so control
/// characters (log-injection vector) are replaced by spaces and the length is
/// capped. Implementations of `ReloadValidator` and `PreCommitCheck` should
/// avoid embedding raw configuration values in reasons regardless — this
/// guard only bounds the message, it cannot redact secrets.
///
/// Deliberately **not** feature-gated: `WatcherGuard::shutdown` (available
/// with plain `watch`) sanitizes its join-error message through this too.
pub(crate) fn flatten_reason(reason: &str) -> String {
    const MAX_REASON_CHARS: usize = 200;
    let flat: String = reason
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let mut bounded: String = flat.chars().take(MAX_REASON_CHARS).collect();
    if flat.chars().count() > MAX_REASON_CHARS {
        bounded.push('…');
    }
    bounded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_reason_bounds_and_single_lines() {
        assert_eq!(flatten_reason("plain"), "plain");
        // Control characters (log-injection vector) become spaces.
        assert_eq!(flatten_reason("a\nb\rc\td"), "a b c d");
        // Length is capped, with the truncation made visible.
        let flat = flatten_reason(&"x".repeat(500));
        assert_eq!(flat.chars().count(), 201);
        assert!(flat.ends_with('…'));
    }
}
