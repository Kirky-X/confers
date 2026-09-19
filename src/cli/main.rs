// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Confers CLI entry point
//!
//! # Exit codes
//!
//! - `0` — success
//! - `1` — configuration error (parse, validation, missing field, …)
//! - `2` — I/O error (file not found, permission denied, …)
fn main() {
    // Initialize i18n first so every user-facing message (including clap
    // help/about and error output) follows the detected locale.
    confers::i18n::init();
    match confers::cli::run::<serde_json::Value>() {
        Ok(()) => {}
        Err(e) => {
            eprintln!(
                "{}: {}",
                confers::i18n::tr("cli-error-prefix"),
                localized_chain(&e)
            );
            // Exit code 2 if the root cause is an I/O error, 1 otherwise.
            let exit_code = if e.chain().any(|c| c.is::<std::io::Error>()) {
                2
            } else {
                1
            };
            std::process::exit(exit_code);
        }
    }
}

/// Render an anyhow error chain with every `ConfersError` segment localized
/// through the errors catalog; segments from other error types keep their
/// original text. Mirrors anyhow's `{e:#}` alternate format (`a: b: c`).
fn localized_chain(e: &anyhow::Error) -> String {
    use confers::i18n::I18nExt;
    e.chain()
        .map(|cause| match cause.downcast_ref::<confers::ConfersError>() {
            Some(config_err) => config_err.to_localized_string(),
            None => cause.to_string(),
        })
        .collect::<Vec<_>>()
        .join(": ")
}
