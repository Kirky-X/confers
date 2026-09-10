// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Confers CLI entry point
//!
//! # Exit codes
//!
//! - `0` — success
//! - `1` — configuration error (parse, validation, missing field, …)
//! - `2` — I/O error (file not found, permission denied, …)

fn main() {
    match confers::cli::run::<serde_json::Value>() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {e:#}");
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
