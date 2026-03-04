//! Fuzz target for parser - tests TOML/JSON/YAML parsing with random inputs.

#![no_main]

use libfuzzer_sys::fuzz_target;
use confers::loader::{parse_content, Format};

fuzz_target!(|data: &[u8]| {
    // Skip empty or very short inputs
    if data.len() < 2 {
        return;
    }

    // Try parsing as TOML
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_content(s, Format::Toml);
    }

    // Try parsing as JSON
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_content(s, Format::Json);
    }

    // Try parsing as YAML
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_content(s, Format::Yaml);
    }
});
