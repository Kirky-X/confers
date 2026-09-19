// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

use super::locale::current_locale;

// ============================================================================
// Embedded FTL resources (compile-time mirror of the locales/ directory)
// ============================================================================

pub const EN_ERRORS_FTL: &str = include_str!("../../locales/en/errors.ftl");
pub const EN_CLI_FTL: &str = include_str!("../../locales/en/cli.ftl");
pub const ZH_ERRORS_FTL: &str = include_str!("../../locales/zh/errors.ftl");
pub const ZH_CLI_FTL: &str = include_str!("../../locales/zh/cli.ftl");

/// All embedded FTL resources as `(crate-relative path, content)` pairs.
///
/// Kept in lockstep with the on-disk `locales/` directory by the sync guard
/// test (`test_embedded_locales_cover_locales_dir`).
pub const EMBEDDED_LOCALES: &[(&str, &str)] = &[
    ("locales/en/errors.ftl", EN_ERRORS_FTL),
    ("locales/en/cli.ftl", EN_CLI_FTL),
    ("locales/zh/errors.ftl", ZH_ERRORS_FTL),
    ("locales/zh/cli.ftl", ZH_CLI_FTL),
];

const EN_FTLS: &[&str] = &[EN_ERRORS_FTL, EN_CLI_FTL];
const ZH_FTLS: &[&str] = &[ZH_ERRORS_FTL, ZH_CLI_FTL];

// ============================================================================
// Translation entry points
// ============================================================================

/// Translate a message key to the current locale.
///
/// Looks up `key` in the Fluent catalog for the current locale, formats any
/// `{ $var }` placeholders with `args`, and falls back to English, then to
/// the key itself (never panics).
pub fn translate(key: &str, args: &[(&str, String)]) -> String {
    let lang = current_locale().language.as_str().to_string();
    format_from_bundle(&lang, key, args)
        .or_else(|| format_from_bundle("en", key, args))
        .unwrap_or_else(|| key.to_string())
}

/// Translate a message key to English specifically, regardless of locale.
pub fn translate_en(key: &str, args: &[(&str, String)]) -> String {
    format_from_bundle("en", key, args).unwrap_or_else(|| key.to_string())
}

/// Shorthand for [`translate()`].
pub fn t(key: &str, args: &[(&str, String)]) -> String {
    translate(key, args)
}

/// Convenience: translate with no dynamic arguments.
pub fn t_simple(key: &str) -> String {
    translate(key, &[])
}

/// Pre-build both language bundles (warm-up for [`super::init()`]).
pub fn warm_up() {
    let _ = EN_BUNDLE.get_or_init(|| build_bundle("en", EN_FTLS));
    let _ = ZH_BUNDLE.get_or_init(|| build_bundle("zh", ZH_FTLS));
}

// ============================================================================
// Fluent bundle management (concurrent bundles: Send + Sync, static-safe)
// ============================================================================

static EN_BUNDLE: OnceLock<FluentBundle<FluentResource>> = OnceLock::new();
static ZH_BUNDLE: OnceLock<FluentBundle<FluentResource>> = OnceLock::new();

/// Format a message from the Fluent catalog for the given language.
///
/// Unknown languages resolve to the `en` bundle; a missing key yields `None`
/// (callers fall back, ultimately to the key itself).
fn format_from_bundle(lang: &str, key: &str, args: &[(&str, String)]) -> Option<String> {
    let bundle = match lang {
        "zh" => ZH_BUNDLE.get_or_init(|| build_bundle("zh", ZH_FTLS)),
        _ => EN_BUNDLE.get_or_init(|| build_bundle("en", EN_FTLS)),
    };

    let msg = bundle.get_message(key)?;
    let pattern = msg.value()?;

    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        fluent_args.set(*name, FluentValue::from(value.clone()));
    }

    let mut errors = vec![];
    let result = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
    Some(result.to_string())
}

/// Build a concurrent bundle for `lang` from the given FTL sources.
fn build_bundle(lang: &str, ftls: &[&str]) -> FluentBundle<FluentResource> {
    let langid: LanguageIdentifier = lang
        .parse()
        .unwrap_or_else(|_| "en".parse().expect("'en' is a valid language identifier"));
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    bundle.set_use_isolating(false);
    for ftl in ftls {
        // Parse errors keep the partially-parsed resource instead of panicking
        // (missing keys then fall back at lookup time).
        let resource = FluentResource::try_new((*ftl).to_string()).unwrap_or_else(|e| e.0);
        bundle
            .add_resource(resource)
            .expect("FTL resources must not declare conflicting message ids");
    }
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Bundle formatting (language pinned per call — no global state, safe
    // under parallel test execution)
    // =========================================================================

    #[test]
    fn test_format_en_simple() {
        assert_eq!(
            format_from_bundle("en", "cli-about", &[]),
            Some("Configuration diagnostics tool for confers".to_string())
        );
        assert_eq!(
            format_from_bundle(
                "en",
                "error-timeout",
                &[("duration_ms", "5000".to_string())]
            ),
            Some("Operation timed out after 5000ms".to_string())
        );
    }

    #[test]
    fn test_format_zh_simple() {
        assert_eq!(
            format_from_bundle("zh", "cli-about", &[]),
            Some("confers 配置诊断工具".to_string())
        );
        assert_eq!(
            format_from_bundle(
                "zh",
                "error-timeout",
                &[("duration_ms", "5000".to_string())]
            ),
            Some("操作在 5000ms 后超时".to_string())
        );
    }

    #[test]
    fn test_format_en_and_zh_with_args() {
        assert_eq!(
            format_from_bundle(
                "en",
                "error-multi-source-detail",
                &[("failed", "1".to_string()), ("total", "3".to_string())]
            ),
            Some("multiple sources failed: 1/3".to_string())
        );
        assert_eq!(
            format_from_bundle(
                "zh",
                "error-module-not-found",
                &[("group", "g1".to_string()), ("module", "m1".to_string())]
            ),
            Some("组 'g1' 中未找到模块 'm1'".to_string())
        );
    }

    // =========================================================================
    // Fallback chain
    // =========================================================================

    #[test]
    fn test_unknown_language_falls_back_to_en_bundle() {
        assert_eq!(
            format_from_bundle("ar", "cli-about", &[]),
            Some("Configuration diagnostics tool for confers".to_string())
        );
        assert_eq!(
            format_from_bundle("fr_FR", "cli-inspect-title", &[]),
            Some("Configuration Inspection".to_string())
        );
    }

    #[test]
    fn test_missing_key_returns_none_from_bundle() {
        assert_eq!(format_from_bundle("en", "no-such-key", &[]), None);
        assert_eq!(format_from_bundle("zh", "no-such-key", &[]), None);
    }

    #[test]
    fn test_translate_missing_key_returns_key_itself_without_panic() {
        // Locale-independent: the key is absent from every bundle.
        assert_eq!(translate("no-such-key", &[]), "no-such-key");
        assert_eq!(t_simple("still-missing"), "still-missing");
    }

    #[test]
    fn test_translate_en_is_always_english() {
        assert_eq!(
            translate_en("error-io", &[("message", "disk full".to_string())]),
            "IO error: disk full"
        );
    }

    // =========================================================================
    // Catalog integrity guards
    // =========================================================================

    /// Extract message ids from FTL source (`key = pattern` lines).
    fn ftl_keys(ftl: &str) -> std::collections::BTreeSet<&str> {
        ftl.lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .filter_map(|line| line.split_once('='))
            .map(|(key, _)| key.trim())
            .filter(|key| !key.is_empty() && !key.contains(' '))
            .collect()
    }

    #[test]
    fn test_en_zh_key_parity() {
        for (en, zh) in [(EN_ERRORS_FTL, ZH_ERRORS_FTL), (EN_CLI_FTL, ZH_CLI_FTL)] {
            let en_keys = ftl_keys(en);
            let zh_keys = ftl_keys(zh);
            assert!(!en_keys.is_empty(), "en catalog must not be empty");
            let missing_in_zh: Vec<_> = en_keys.difference(&zh_keys).collect();
            let missing_in_en: Vec<_> = zh_keys.difference(&en_keys).collect();
            assert!(
                missing_in_zh.is_empty() && missing_in_en.is_empty(),
                "key mismatch: missing_in_zh={missing_in_zh:?} missing_in_en={missing_in_en:?}"
            );
        }
    }

    #[test]
    fn test_embedded_locales_cover_locales_dir() {
        let locales_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut disk_files: Vec<String> = Vec::new();
        for lang in ["en", "zh"] {
            let entries =
                std::fs::read_dir(locales_dir.join(lang)).expect("locales/{lang} must exist");
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("ftl") {
                    disk_files.push(format!(
                        "locales/{lang}/{}",
                        entry.file_name().to_string_lossy()
                    ));
                }
            }
        }
        disk_files.sort();

        let embedded: std::collections::BTreeSet<&str> =
            EMBEDDED_LOCALES.iter().map(|(path, _)| *path).collect();
        let disk: std::collections::BTreeSet<&str> =
            disk_files.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            embedded, disk,
            "EMBEDDED_LOCALES and the locales/ directory must list exactly the same files"
        );

        for (path, content) in EMBEDDED_LOCALES {
            let rel = path
                .strip_prefix("locales/")
                .expect("paths are locales/-relative");
            let on_disk = std::fs::read_to_string(locales_dir.join(rel))
                .unwrap_or_else(|e| panic!("read {path}: {e}"));
            assert_eq!(
                content, &on_disk,
                "embedded {path} has drifted from the file on disk"
            );
        }
    }
}
