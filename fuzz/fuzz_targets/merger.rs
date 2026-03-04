//! Fuzz target for merger - tests merge operations with random inputs.

#![no_main]

use libfuzzer_sys::fuzz_target;
use confers::{AnnotatedValue, ConfigValue, MergeStrategy, SourceId};
use std::sync::Arc;

/// Generate a random-ish ConfigValue from input bytes.
fn generate_value(data: &[u8]) -> Option<ConfigValue> {
    if data.is_empty() {
        return None;
    }

    match data[0] % 4 {
        0 => Some(ConfigValue::Null),
        1 => Some(ConfigValue::Bool(data.len() % 2 == 0)),
        2 => {
            let n = data.iter().fold(0u64, |acc, &b| acc.wrapping_add(b as u64));
            Some(ConfigValue::I64(n as i64))
        }
        _ => {
            let s = format!("fuzz_{}", data.len());
            Some(ConfigValue::String(Arc::from(s)))
        }
    }
}

/// Create an AnnotatedValue from input bytes.
fn create_annotated(data: &[u8]) -> Option<AnnotatedValue> {
    let value = generate_value(data)?;
    Some(AnnotatedValue::new(
        value,
        SourceId::new("fuzz"),
        "fuzz_path",
    ))
}

fuzz_target!(|data: &[u8]| {
    if let (Some(low), Some(high)) = (create_annotated(data), create_annotated(&data[data.len()/2..])) {
        // Test Replace strategy
        let _ = confers::merger::merge(&low, &high, MergeStrategy::Replace);

        // Test DeepMerge strategy (should handle gracefully even if it fails)
        let _ = confers::merger::merge(&low, &high, MergeStrategy::DeepMerge);
    }
});
