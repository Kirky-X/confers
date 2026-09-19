// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

#![no_main]

use confers::{Format, SourceId, parse_content};
use libfuzzer_sys::fuzz_target;
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let source = SourceId::new("fuzz");

    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_content(s, Format::Toml, source.clone(), None::<&Path>);
    }

    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_content(s, Format::Json, source.clone(), None::<&Path>);
    }

    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_content(s, Format::Yaml, source, None::<&Path>);
    }
});
