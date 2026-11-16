//! The result of rendering one document.
//!
//! Pattern: Value object — plain data for the page frame and the site builder.

use crate::report::Report;

/// One entry of the table of contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TocEntry {
    /// Heading level 1 to 6.
    pub level: u8,
    /// The id of the heading in the body.
    pub id: String,
    /// Plain text of the heading.
    pub text: String,
}

/// A rendered document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HtmlPage {
    /// The HTML fragment (no `<html>`, no `<head>`).
    pub body: String,
    /// Title: the front matter `title`, else the first level-1 heading.
    pub title: Option<String>,
    /// Front matter `description`.
    pub description: Option<String>,
    /// Front matter `lang`.
    pub lang: Option<String>,
    /// Headings in order.
    pub toc: Vec<TocEntry>,
    /// How many level-1 headings the body has.
    pub top_headings: usize,
    /// Link addresses written, in order of first use.
    pub links: Vec<String>,
    /// Media addresses written, in order of first use.
    pub media: Vec<String>,
    /// What the export could not do exactly.
    pub report: Report,
}

/// `title`, `description` and `lang` of a front matter block (flat `key: value` lines).
#[must_use]
pub fn front_matter(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let get = |key: &str| {
        text.lines().find_map(|l| {
            let v = l.strip_prefix(key)?.strip_prefix(':')?.trim();
            let v = v
                .strip_prefix('"')
                .and_then(|x| x.strip_suffix('"'))
                .or_else(|| v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
                .unwrap_or(v);
            (!v.is_empty()).then(|| v.to_string())
        })
    };
    (get("title"), get("description"), get("lang"))
}
