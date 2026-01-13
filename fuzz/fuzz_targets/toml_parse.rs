#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test UTF-8 validation (core functionality)
    let _ = std::str::from_utf8(data);

    // Test basic string operations that don't require external crates
    if let Ok(s) = std::str::from_utf8(data) {
        // Test length operations
        let _ = s.len();
        let _ = s.is_empty();
        let _ = s.trim();
        let _ = s.to_lowercase();
        let _ = s.to_uppercase();

        // Test character operations
        let _ = s.chars().next();
        let _ = s.bytes().next();

        // Test split operations
        let _ = s.split_whitespace().next();
        let _ = s.split(',').next();
    }
});
