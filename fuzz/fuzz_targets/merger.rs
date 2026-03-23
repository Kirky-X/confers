#![no_main]

use confers::merger::{MergeEngine, MergeStrategy};
use confers::{AnnotatedValue, ConfigValue, SourceId};
use libfuzzer_sys::fuzz_target;

fn generate_value(data: &[u8]) -> Option<ConfigValue> {
    if data.is_empty() {
        return None;
    }

    match data[0] % 4 {
        0 => Some(ConfigValue::Null),
        1 => Some(ConfigValue::Bool(data.len().is_multiple_of(2))),
        2 => {
            let n = data.iter().fold(0u64, |acc, &b| acc.wrapping_add(b as u64));
            Some(ConfigValue::I64(n as i64))
        }
        _ => {
            let s = format!("fuzz_{}", data.len());
            Some(ConfigValue::String(s))
        }
    }
}

fn create_annotated(data: &[u8]) -> Option<AnnotatedValue> {
    let value = generate_value(data)?;
    Some(AnnotatedValue::new(
        value,
        SourceId::new("fuzz"),
        "fuzz_path",
    ))
}

fuzz_target!(|data: &[u8]| {
    if let (Some(low), Some(high)) = (
        create_annotated(data),
        create_annotated(&data[data.len() / 2..]),
    ) {
        let engine_replace = MergeEngine::new().with_default_strategy(MergeStrategy::Replace);
        let _ = engine_replace.merge(&low, &high);

        let engine_deep = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);
        let _ = engine_deep.merge(&low, &high);
    }
});
