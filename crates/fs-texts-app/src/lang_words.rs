//! The few sentences of the language bar and its notices, in German and English.
//!
//! Pattern: Adapter over `UiTexts` — the sentences are `MessageFormat` 2 (`locales/`, ADR 0057: `lang-*`; the name of a language
//! is chosen by a selector on its code); a language the files do not have gets English. A sentence is whole, with named
//! places, never built from pieces.

use crate::ui_texts::UiTexts;

fn texts(ui: &str) -> UiTexts {
    UiTexts::new(ui == "de")
}

/// The name of a language in that language.
pub fn language_name(code: &str) -> String {
    UiTexts::new(code == "de").say("lang-own", &[("code", code)])
}

/// The name of a language as a person speaking `ui` says it ("auf Englisch", "in German").
fn named_in(ui: &str, code: &str) -> String {
    texts(ui).say("lang-name", &[("code", code)])
}

/// `Language:` / `Sprache:`.
pub fn language_label(ui: &str) -> String {
    texts(ui).plain("lang-label")
}

/// The page is shown in another language than asked for because it has none in that one.
pub fn only_in(ui: &str, shown: &str) -> String {
    texts(ui).say("lang-only-in", &[("lang", &named_in(ui, shown))])
}

/// The shown translation was made from an older state of its origin.
pub fn outdated(ui: &str, origin: &str) -> String {
    texts(ui).say("lang-outdated", &[("lang", &named_in(ui, origin))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sentences_exist_in_both_languages() {
        assert_eq!(only_in("de", "en"), "Diese Seite gibt es nur auf Englisch.");
        assert_eq!(
            only_in("en", "de"),
            "This page is only available in German."
        );
        assert!(outdated("de", "en").contains("veraltet"));
        assert!(outdated("en", "de").contains("outdated"));
        assert_eq!(language_label("de"), "Sprache");
        assert_eq!(language_label("en"), "Language");
    }

    /// The name of a language is chosen by its code; a code the files do not know stays as it is.
    #[test]
    fn a_language_is_named_by_its_code() {
        assert_eq!(language_name("de"), "Deutsch");
        assert_eq!(language_name("en"), "English");
        assert_eq!(language_name("fr"), "fr");
        assert_eq!(named_in("de", "fr"), "fr");
    }
}
