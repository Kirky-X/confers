// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use std::str::FromStr;
use std::sync::{OnceLock, RwLock};

use unic_langid::LanguageIdentifier;

/// Global default locale, initialized once on first access.
static GLOBAL_LOCALE: OnceLock<LanguageIdentifier> = OnceLock::new();

/// Override locale set via [`set_locale()`].
static OVERRIDE_LOCALE: RwLock<Option<LanguageIdentifier>> = RwLock::new(None);

/// Warm up the locale subsystem: run the detection chain once and cache the
/// result. Idempotent; safe to call at every process/CLI entry point.
pub fn init() {
    let _ = current_locale();
}

/// Get the current locale.
///
/// Resolution order:
/// 1. Override set via [`set_locale()`]
/// 2. Global default (detected once on first access via `detect_locale()`)
pub fn current_locale() -> LanguageIdentifier {
    if let Ok(guard) = OVERRIDE_LOCALE.read()
        && let Some(locale) = guard.as_ref()
    {
        return locale.clone();
    }
    GLOBAL_LOCALE.get_or_init(detect_locale).clone()
}

/// Set the global locale override (e.g. from a `--lang` CLI argument).
///
/// After calling this, [`current_locale()`] returns the given locale until
/// [`clear_locale_override()`] is called. Only `en`/`zh` matter for message
/// lookup; other well-formed BCP-47 tags are accepted here but fall back to
/// the `en` bundle at format time.
pub fn set_locale(locale: &str) -> Result<(), String> {
    let parsed: LanguageIdentifier = locale
        .parse()
        .map_err(|e| format!("invalid locale '{locale}': {e}"))?;
    let mut guard = OVERRIDE_LOCALE
        .write()
        .expect("locale override RwLock poisoned");
    *guard = Some(parsed);
    Ok(())
}

/// Clear the locale override, reverting to the auto-detected locale.
pub fn clear_locale_override() {
    let mut guard = OVERRIDE_LOCALE
        .write()
        .expect("locale override RwLock poisoned");
    *guard = None;
}

/// Get the auto-detected locale (ignoring any override).
pub fn detected_locale() -> LanguageIdentifier {
    GLOBAL_LOCALE.get_or_init(detect_locale).clone()
}

/// Run the detection chain against the real process environment.
fn detect_locale() -> LanguageIdentifier {
    detect_from(|key| std::env::var(key).ok(), sys_locale::get_locale())
}

/// Pure detection-chain logic.
///
/// Environment access is injected as a closure and the system locale as a
/// pre-resolved `Option`, so tests can exercise every link of the chain
/// (priority, normalization, fallback) without mutating process state.
fn detect_from(
    get_env: impl Fn(&str) -> Option<String>,
    sys_locale: Option<String>,
) -> LanguageIdentifier {
    // 1. Project override variable.
    if let Some(lang) = get_env("CONFERS_LANG").filter(|v| !v.trim().is_empty())
        && let Some(locale) = normalize(&lang)
    {
        return locale;
    }
    // 2. Explicit POSIX environment chain (read explicitly so behavior is
    //    deterministic on Windows/edge environments too).
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(lang) = get_env(key).filter(|v| !v.trim().is_empty())
            && let Some(locale) = normalize(&lang)
        {
            return locale;
        }
    }
    // 3. System locale detection.
    if let Some(sys) = sys_locale
        && let Some(locale) = normalize(&sys)
    {
        return locale;
    }
    // 4. Terminal fallback: everything ends at English.
    LanguageIdentifier::from_str("en").expect("'en' is a valid language identifier")
}

/// Normalize a raw locale string to `en` or `zh`.
///
/// Strips `@modifier` and `.codeset` suffixes, accepts `_`/`-` separators,
/// maps every `zh*` to `zh` and every `en*` to `en`. Returns `None` for
/// `C`/`POSIX` (chain continues) and for any unsupported language.
fn normalize(raw: &str) -> Option<LanguageIdentifier> {
    let s = raw.split('@').next()?.split('.').next()?.replace('_', "-");
    // C / POSIX mean "unspecified": let the detection chain continue.
    if matches!(s.as_str(), "C" | "POSIX") {
        return None;
    }
    let locale: LanguageIdentifier = s.parse().ok()?;
    match locale.language.as_str() {
        "zh" => Some(LanguageIdentifier::from_str("zh").ok()?),
        "en" => Some(LanguageIdentifier::from_str("en").ok()?),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn env_map<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: BTreeMap<&str, &str> = pairs.iter().copied().collect();
        move |key| map.get(key).map(|v| (*v).to_string())
    }

    // =========================================================================
    // normalize(): zh* -> zh, en* -> en, everything else -> None
    // =========================================================================

    #[test]
    fn test_normalize_zh_variants_collapse_to_zh() {
        for raw in [
            "zh",
            "zh-CN",
            "zh_CN",
            "zh_TW",
            "zh_HK",
            "zh-Hans",
            "zh_SG.UTF-8",
        ] {
            let locale = normalize(raw).expect(raw);
            assert_eq!(locale.language.as_str(), "zh", "normalize({raw})");
            assert_eq!(locale.to_string(), "zh", "zh* must collapse to plain zh");
        }
    }

    #[test]
    fn test_normalize_en_variants_collapse_to_en() {
        for raw in ["en", "en-US", "en_US", "en_GB.UTF-8", "en_US@euro"] {
            let locale = normalize(raw).expect(raw);
            assert_eq!(locale.to_string(), "en", "en* must collapse to plain en");
        }
    }

    #[test]
    fn test_normalize_unsupported_or_invalid_returns_none() {
        for raw in [
            "fr_FR",
            "de-DE",
            "es",
            "ja_JP.UTF-8",
            "C",
            "POSIX",
            "C.UTF-8",
            "",
            "garbage!!",
            "12345",
        ] {
            assert!(normalize(raw).is_none(), "normalize({raw}) must be None");
        }
    }

    // =========================================================================
    // detect_from(): chain order and fallbacks (pure — no process env access)
    // =========================================================================

    #[test]
    fn test_detect_from_lang_zh_cn_utf8() {
        let locale = detect_from(env_map(&[("LANG", "zh_CN.UTF-8")]), None);
        assert_eq!(locale.to_string(), "zh");
    }

    #[test]
    fn test_detect_from_lc_all_zh_tw() {
        let locale = detect_from(env_map(&[("LC_ALL", "zh_TW")]), None);
        assert_eq!(locale.to_string(), "zh");
    }

    #[test]
    fn test_detect_from_lc_messages_english() {
        let locale = detect_from(env_map(&[("LC_MESSAGES", "en_GB.UTF-8")]), None);
        assert_eq!(locale.to_string(), "en");
    }

    #[test]
    fn test_detect_from_unsupported_language_falls_back_to_en() {
        let locale = detect_from(env_map(&[("LANG", "fr_FR.UTF-8")]), None);
        assert_eq!(locale.to_string(), "en");
    }

    #[test]
    fn test_detect_from_c_posix_falls_back_to_en() {
        // C/POSIX skip to the next chain link; nothing else set -> en.
        let locale = detect_from(env_map(&[("LANG", "C"), ("LC_ALL", "POSIX")]), None);
        assert_eq!(locale.to_string(), "en");
    }

    #[test]
    fn test_detect_from_empty_and_malformed_fall_back_to_en() {
        let locale = detect_from(env_map(&[("LANG", "   "), ("LC_ALL", "")]), None);
        assert_eq!(locale.to_string(), "en");
        let locale = detect_from(env_map(&[("LANG", "@@@")]), None);
        assert_eq!(locale.to_string(), "en");
    }

    #[test]
    fn test_detect_from_confers_lang_overrides_lc_all() {
        let locale = detect_from(
            env_map(&[("CONFERS_LANG", "zh-CN"), ("LC_ALL", "en_US.UTF-8")]),
            None,
        );
        assert_eq!(locale.to_string(), "zh");

        let locale = detect_from(
            env_map(&[("CONFERS_LANG", "en"), ("LC_ALL", "zh_CN.UTF-8")]),
            None,
        );
        assert_eq!(locale.to_string(), "en");
    }

    #[test]
    fn test_detect_from_lc_all_overrides_lang() {
        let locale = detect_from(env_map(&[("LC_ALL", "zh_CN"), ("LANG", "en_US")]), None);
        assert_eq!(locale.to_string(), "zh");
    }

    #[test]
    fn test_detect_from_env_lang_beats_system_locale() {
        let locale = detect_from(
            env_map(&[("LANG", "en_US.UTF-8")]),
            Some("zh_CN.UTF-8".into()),
        );
        assert_eq!(locale.to_string(), "en");
        // System locale is used only when the env chain yields nothing.
        let locale = detect_from(env_map(&[]), Some("zh_CN".into()));
        assert_eq!(locale.to_string(), "zh");
    }

    #[test]
    fn test_detect_from_chain_always_ends_en_or_zh() {
        // Domain check: whatever the inputs, the result is exactly en or zh.
        for lang in ["C.UTF-8", "fr", "", "zz_ZZ", "EN_us", "ZH_tw"] {
            let locale = detect_from(env_map(&[("LANG", lang)]), Some(lang.to_string()));
            assert!(matches!(locale.to_string().as_str(), "en" | "zh"));
        }
    }

    // =========================================================================
    // Global override state (serialized into a single #[ignore] test to avoid
    // parallel-test races on the process-global override slot)
    // =========================================================================

    #[test]
    #[ignore] // mutates the global locale override; run explicitly
    fn test_set_locale_override_roundtrip() {
        clear_locale_override();
        set_locale("zh-CN").expect("zh-CN is valid");
        assert_eq!(current_locale().to_string(), "zh");
        clear_locale_override();
        assert_ne!(current_locale().to_string(), "zh");
        assert!(set_locale("not a locale!!!").is_err());
        clear_locale_override();
    }

    #[test]
    fn test_clear_locale_override_is_idempotent() {
        clear_locale_override();
        clear_locale_override();
    }

    #[test]
    fn test_current_locale_is_en_or_zh_domain() {
        // No override here (other tests clear it); detected locale must be in
        // the supported domain regardless of the host environment.
        clear_locale_override();
        let locale = current_locale();
        assert!(matches!(locale.to_string().as_str(), "en" | "zh"));
    }
}
