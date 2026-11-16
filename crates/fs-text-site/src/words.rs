//! The words of the pages the builder writes itself (ADR 0057): whole labels, chosen by language.
//!
//! Pattern: none (constants).

/// Labels of generated pages.
#[derive(Clone, Copy, Debug)]
pub struct SiteWords {
    /// Address part and title of the full contents page.
    pub contents_slug: &'static str,
    /// Title of the contents page and label in the navigation.
    pub contents: &'static str,
    /// Label of the sitemap link in the navigation.
    pub sitemap: &'static str,
    /// Label of the hint where the navigation was cut.
    pub more: &'static str,
    /// Title of the page "not found".
    pub not_found: &'static str,
    /// Text of the page "not found".
    pub not_found_text: &'static str,
    /// Link back to the start.
    pub to_start: &'static str,
    /// Lead of the language choice.
    pub choose_language: &'static str,
}

const DE: SiteWords = SiteWords {
    contents_slug: "inhalt",
    contents: "Inhalt",
    sitemap: "Sitemap",
    more: "mehr …",
    not_found: "Seite nicht gefunden",
    not_found_text: "Diese Seite gibt es nicht (mehr).",
    to_start: "Zur Startseite",
    choose_language: "Sprache wählen",
};

const EN: SiteWords = SiteWords {
    contents_slug: "contents",
    contents: "Contents",
    sitemap: "Sitemap",
    more: "more …",
    not_found: "Page not found",
    not_found_text: "This page does not exist (any more).",
    to_start: "To the start page",
    choose_language: "Choose a language",
};

impl SiteWords {
    /// The words for a language tag; unknown languages get English.
    #[must_use]
    pub fn for_lang(lang: &str) -> SiteWords {
        match lang.split('-').next().unwrap_or("") {
            "de" => DE,
            _ => EN,
        }
    }
}
