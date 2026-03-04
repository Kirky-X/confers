//! Fuzz target for interpolation - tests variable interpolation with random inputs.

#![no_main]

use libfuzzer_sys::fuzz_target;

#[cfg(feature = "interpolation")]
fuzz_target!(|data: &[u8]| {
    if let Ok(template) = std::str::from_utf8(data) {
        // Simple variable lookup that returns None for any variable
        let lookup = |_: &str| None;

        // This should not panic and should handle errors gracefully
        let _ = confers::interpolation::interpolate(template, lookup);
    }
});

#[cfg(not(feature = "interpolation"))]
fuzz_target!(|_data: &[u8]| {
    // No-op when interpolation feature is not enabled
});
