// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(template) = std::str::from_utf8(data) {
        let lookup = |_: &str| None;
        let _ = confers::interpolation::interpolate(template, &lookup);
    }
});
