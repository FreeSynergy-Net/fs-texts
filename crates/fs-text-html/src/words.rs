//! The words of generated markup (ADR 0057): one whole sentence or label per entry.
//!
//! Pattern: none (constants, chosen by language).

/// Labels the generated HTML needs. Texts of the document itself are never in here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Words {
    /// Heading of the footnote list.
    pub footnotes: &'static str,
    /// Label of the link from a footnote back to its place in the text.
    pub footnote_back: &'static str,
    /// Label of a task that is open.
    pub task_open: &'static str,
    /// Label of a task that is done.
    pub task_done: &'static str,
    /// Skip link to the main content.
    pub skip: &'static str,
    /// Label of the navigation.
    pub nav: &'static str,
    /// Label of the bread crumb navigation.
    pub breadcrumb: &'static str,
    /// Label of the language bar.
    pub languages: &'static str,
}

const DE: Words = Words {
    footnotes: "Fußnoten",
    footnote_back: "Zurück zur Stelle im Text",
    task_open: "Aufgabe, offen",
    task_done: "Aufgabe, erledigt",
    skip: "Zum Inhalt springen",
    nav: "Seitennavigation",
    breadcrumb: "Pfad zur Seite",
    languages: "Sprachen",
};

const EN: Words = Words {
    footnotes: "Footnotes",
    footnote_back: "Back to the place in the text",
    task_open: "Task, open",
    task_done: "Task, done",
    skip: "Skip to the content",
    nav: "Page navigation",
    breadcrumb: "Path to the page",
    languages: "Languages",
};

impl Words {
    /// The words for a language tag (`de`, `de-AT`, `en`, …); unknown languages get English.
    #[must_use]
    pub fn for_lang(lang: &str) -> Words {
        match lang.split('-').next().unwrap_or("") {
            "de" => DE,
            _ => EN,
        }
    }
}

/// The name of a language in that language (a name, not a sentence), for the language bar.
#[must_use]
pub fn autonym(lang: &str) -> &str {
    match lang {
        "de" => "Deutsch",
        "en" => "English",
        other => other,
    }
}
