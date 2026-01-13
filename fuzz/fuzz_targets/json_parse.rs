#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Test UTF-8 validation (core functionality)
    let _ = std::str::from_utf8(data);

    // Test basic string operations that don't require external crates
    if let Ok(s) = std::str::from_utf8(data) {
        // Test parsing operations
        let _ = s.parse::<u32>();
        let _ = s.parse::<i64>();
        let _ = s.parse::<f64>();

        // Test format operations
        let _ = format!("{}", s);
        let _ = s.to_string();

        // Test search operations
        let _ = s.contains("test");
        let _ = s.starts_with("test");
        let _ = s.ends_with("test");
        let _ = s.find("test");
        let _ = s.rfind("test");
    }
});
