// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use super::catalog;

/// Trait for error types that have localized messages.
///
/// Each variant returns a unique `message_key()` mapping to a translation in
/// the message catalog (`locales/*/errors.ftl`), plus the dynamic arguments
/// for `{ $placeholder }` substitution via `message_args()`.
pub trait LocalizedMsg {
    /// Return the message catalog key for this error variant.
    fn message_key(&self) -> &'static str;

    /// Return the dynamic arguments for message template substitution.
    ///
    /// Each entry is a `(name, value)` pair where `name` corresponds to a
    /// `{ $name }` placeholder in the Fluent template.
    fn message_args(&self) -> Vec<(&str, String)> {
        Vec::new()
    }
}

/// Extension trait providing localized string conversion for errors.
///
/// Automatically implemented for all types that implement both
/// [`LocalizedMsg`] and [`std::error::Error`].
pub trait I18nExt: LocalizedMsg + std::error::Error {
    /// Return the error message translated to the current locale.
    ///
    /// Falls back to English, then to the key itself; never panics.
    fn to_localized_string(&self) -> String {
        catalog::translate(self.message_key(), &self.message_args())
    }

    /// Return the error message in English (the canonical fallback),
    /// regardless of the current locale.
    fn message_en(&self) -> String {
        catalog::translate_en(self.message_key(), &self.message_args())
    }
}

impl<E: LocalizedMsg + std::error::Error> I18nExt for E {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    // Locale-dependent output is asserted against the full {en, zh} domain
    // instead of mutating the global locale, per the parallel-test guidance
    // in the i18n reference pattern. Language-pinned assertions live in the
    // catalog tests.

    #[derive(Debug)]
    struct CatalogBackedError;

    impl fmt::Display for CatalogBackedError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // Canonical English Display (thiserror-style, hardcoded).
            write!(f, "remote configuration source unavailable")
        }
    }

    impl std::error::Error for CatalogBackedError {}

    impl LocalizedMsg for CatalogBackedError {
        fn message_key(&self) -> &'static str {
            "error-remote-unavailable"
        }
    }

    #[test]
    fn test_message_key() {
        assert_eq!(CatalogBackedError.message_key(), "error-remote-unavailable");
    }

    #[test]
    fn test_message_args_default_empty() {
        assert!(CatalogBackedError.message_args().is_empty());
    }

    #[test]
    fn test_message_en_is_always_english() {
        assert_eq!(
            CatalogBackedError.message_en(),
            "Remote configuration source unavailable"
        );
    }

    #[test]
    fn test_to_localized_string_stays_in_supported_domain() {
        let localized = CatalogBackedError.to_localized_string();
        assert!(
            matches!(
                localized.as_str(),
                "Remote configuration source unavailable" | "远程配置源不可用"
            ),
            "localized output must come from the catalog: {localized}"
        );
    }

    #[test]
    fn test_canonical_display_is_untouched_by_i18n() {
        // The dual-track contract: Display stays the English canonical form.
        assert_eq!(
            CatalogBackedError.to_string(),
            "remote configuration source unavailable"
        );
    }
}
