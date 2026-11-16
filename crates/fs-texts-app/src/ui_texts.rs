//! The words of the wiki window, from `MessageFormat` 2 files (ADR 0057): menus, dialogs, answers, the words of the layouts
//! and of the reader, and those of the Admin window.
//!
//! Pattern: Factory over embedded `locales/en.mf2` and `locales/de.mf2` — one `UiTexts` per interface language; a sentence is
//! whole with named holes (`{$path}`), never built from pieces; a word that depends on a number or a code is chosen by a
//! selector (`.match`), never by code. `check_pairs(TEXTS)` must be empty (a test asserts it), so no id exists in one
//! language only. The words of the layouts and of the reader are not kept here: they are those of `fs-components`.
//! The words of `words.rs` (English, for the console) are still to follow.

use fs_components_mf2::{Mf2Texts, Source};
use fs_components_words::{Args, Locale, Localized, TextId, Words};
use fs_components_workbenches::{Labels, WikiLabels};

/// The two language files of the window.
pub const TEXTS: &[Source] = &[
    Source {
        language: "en",
        text: include_str!("locales/en.mf2"),
    },
    Source {
        language: "de",
        text: include_str!("locales/de.mf2"),
    },
];

/// The words of one interface language.
pub struct UiTexts {
    texts: Mf2Texts,
    locale: Locale,
}

impl UiTexts {
    /// German or English.
    pub fn new(german: bool) -> Self {
        let locale =
            Locale::parse(if german { "de" } else { "en" }).unwrap_or_else(|_| Locale::english());
        Self {
            texts: Mf2Texts::new(TEXTS, &locale),
            locale,
        }
    }

    /// The language these words are in.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// The sentence `id` without holes.
    pub fn plain(&self, id: &str) -> String {
        self.texts.plain(id)
    }

    /// The sentence `id`, its holes filled with `values` (`(name, text)`).
    pub fn say(&self, id: &str, values: &[(&str, &str)]) -> String {
        let args = values
            .iter()
            .fold(Args::new(), |args, (name, value)| args.text(name, *value));
        self.texts.say(&TextId::new(id), &args).text().to_owned()
    }

    /// The sentence `id` for the number `n` (its plural form), its holes filled with `values` (`(name, text)`); the number
    /// is `$count` in the message.
    pub fn say_count(&self, id: &str, n: u64, values: &[(&str, &str)]) -> String {
        let args = values
            .iter()
            .fold(Args::new().count("count", n), |args, (name, value)| {
                args.text(name, *value)
            });
        self.texts.say(&TextId::new(id), &args).text().to_owned()
    }

    /// The words of the wiki layouts: `fs-components` owns them (one place for every program, MF2 in both languages, the page
    /// count with its plural selector), so the wiki does not keep a copy.
    pub fn wiki_labels(&self) -> WikiLabels {
        WikiLabels::for_locale(&self.locale)
    }

    /// The words of the reader (the plain frontend, the editor, the notices): `fs-components` owns them, as above.
    pub fn reader_labels(&self) -> Labels {
        Labels::for_locale(&self.locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_is_in_german_and_english_with_the_same_holes() {
        assert_eq!(fs_components_mf2::check_pairs(TEXTS), Vec::<String>::new());
    }

    /// Finding W3 of 2026-11-14: "1 Seiten". Every text that shows a number says one and many differently.
    #[test]
    fn count_texts_say_one_and_many_in_both_languages() {
        let (de, en) = (UiTexts::new(true), UiTexts::new(false));
        let said = |t: &UiTexts| {
            let w = t.wiki_labels();
            (w.pages_said(0), w.pages_said(1), w.pages_said(2))
        };
        assert_eq!(
            said(&de),
            ("0 Seiten".into(), "1 Seite".into(), "2 Seiten".into())
        );
        assert_eq!(
            said(&en),
            ("0 pages".into(), "1 page".into(), "2 pages".into())
        );
        // The delete question counts the page and its subpages.
        let sub = |t: &UiTexts, n| t.say_count("dlg-delete-body-sub", n, &[]);
        assert!(
            sub(&de, 2).contains("insgesamt 2 Seiten"),
            "{}",
            sub(&de, 2)
        );
        assert!(
            sub(&de, 1).contains("insgesamt 1 Seite)"),
            "{}",
            sub(&de, 1)
        );
        assert!(sub(&en, 2).contains("(2 pages in all)"), "{}", sub(&en, 2));
        assert!(sub(&en, 1).contains("(1 page in all)"), "{}", sub(&en, 1));
    }

    /// One place for the words: the wiki's own files hold no id of the layouts (`wk-`) or of the reader (`rd-`), whose words
    /// `fs-components` owns. A copy in `locales/` would drift from it at every lift.
    #[test]
    fn the_words_of_the_components_are_not_copied_into_the_wiki_files() {
        for source in TEXTS {
            for line in source.text.lines() {
                assert!(
                    !line.starts_with("wk-") && !line.starts_with("rd-"),
                    "{}: {line}",
                    source.language
                );
            }
        }
    }

    #[test]
    fn the_layout_and_reader_words_follow_the_language() {
        let (de, en) = (UiTexts::new(true), UiTexts::new(false));
        assert_eq!(de.wiki_labels().pages, "Seiten");
        assert_eq!(en.wiki_labels().pages, "Pages");
        assert_eq!(de.wiki_labels().page_count, "{} Seiten");
        assert_eq!(en.reader_labels().save, "Save");
        assert_eq!(de.reader_labels().save, "Speichern");
        assert_eq!(
            de.reader_labels().nothing_found,
            "Nichts gefunden für „{}“."
        );
        assert_eq!(de.wiki_labels().title_hint, "Ohne Titel");
    }

    #[test]
    fn a_hole_is_filled_and_no_word_is_missing() {
        let de = UiTexts::new(true);
        assert_eq!(
            de.say("err-taken", &[("name", "A")]),
            "Eine Seite „A“ gibt es schon."
        );
        // A missing id would come back as itself; none of the ids the window asks for may.
        let ids = [
            "menu-page",
            "page-new",
            "dlg-keep",
            "err-empty",
            "admin-type-trash",
            "lang-label",
        ];
        for id in ids {
            for ui in [&de, &UiTexts::new(false)] {
                assert_ne!(ui.plain(id), id, "{id}");
            }
        }
    }
}
