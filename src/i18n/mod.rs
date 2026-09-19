// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

pub mod catalog;
pub mod error_ext;
pub mod locale;

// Re-export the primary entry points at module level.
pub use catalog::{t, t_simple, translate, translate_en};
pub use error_ext::{I18nExt, LocalizedMsg};
pub use locale::{clear_locale_override, current_locale, detected_locale, set_locale};

/// Translate a message key with no dynamic arguments.
///
/// Convenience wrapper over [`translate()`]; falls back to English, then to
/// the key itself. Safe to call before [`init()`].
pub fn tr(key: &str) -> String {
    translate(key, &[])
}

/// Translate a message key with dynamic `{ $placeholder }` arguments.
pub fn tr_args(key: &str, args: &[(&str, String)]) -> String {
    translate(key, args)
}

/// Initialize the i18n subsystem: run the locale detection chain once and
/// pre-build both language bundles so the first user-visible message pays no
/// parse cost. Idempotent; call at every process/CLI entry point before any
/// user-facing output.
pub fn init() {
    locale::init();
    catalog::warm_up();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tr_and_tr_args_are_catalog_backed() {
        assert_eq!(
            tr("cli-inspect-title"),
            catalog::t_simple("cli-inspect-title")
        );
        assert_eq!(
            tr_args("cli-inspect-loaded-sources", &[("count", "3".to_string())]),
            translate("cli-inspect-loaded-sources", &[("count", "3".to_string())])
        );
    }

    #[test]
    fn test_tr_missing_key_returns_key_without_panic() {
        assert_eq!(tr("definitely-not-a-key"), "definitely-not-a-key");
    }

    #[test]
    fn test_init_is_idempotent_and_does_not_panic() {
        init();
        init();
    }

    #[test]
    fn test_locale_exports_present() {
        // The detected/overridden locale must always be in the {en, zh} domain.
        clear_locale_override();
        let locale = current_locale().to_string();
        assert!(matches!(locale.as_str(), "en" | "zh"));
    }
}
