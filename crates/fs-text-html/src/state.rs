//! What one page render collects while it runs.
//!
//! Pattern: Value object (mutable accumulator) — ids, footnote numbers, used links and media,
//! and the report; handed down by `&mut` so the walkers stay pure otherwise.

use crate::raw::RawHtmlPolicy;
use crate::report::Report;
use crate::resolve::{LinkResolver, MediaResolver};
use crate::slug::Ids;
use crate::words::Words;

/// The strategies and settings of a render (read only).
pub struct Env<'a> {
    /// Link targets.
    pub links: &'a dyn LinkResolver,
    /// Media targets.
    pub media: &'a dyn MediaResolver,
    /// Raw HTML policy.
    pub raw: RawHtmlPolicy,
    /// Words of generated markup.
    pub words: Words,
}

/// The collected state of one page.
#[derive(Default)]
pub struct PageState {
    /// Heading ids.
    pub ids: Ids,
    /// What was noticed.
    pub report: Report,
    /// Footnote labels in order of first reference (number = index + 1).
    pub notes: Vec<String>,
    /// Labels that have a definition.
    pub defined: Vec<String>,
    /// How often each footnote was referenced (parallel to `notes`).
    pub refs: Vec<u32>,
    /// Link addresses written, in order, without repeats.
    pub links: Vec<String>,
    /// Media addresses written, in order, without repeats.
    pub media: Vec<String>,
}

impl PageState {
    /// Number of footnote `label`; a new label gets the next number. `None` for a label with no
    /// definition.
    pub fn note_number(&mut self, label: &str) -> Option<(usize, u32)> {
        if !self.defined.iter().any(|d| d == label) {
            return None;
        }
        let i = if let Some(i) = self.notes.iter().position(|n| n == label) {
            i
        } else {
            self.notes.push(label.to_string());
            self.refs.push(0);
            self.notes.len() - 1
        };
        self.refs[i] += 1;
        Some((i + 1, self.refs[i]))
    }
    /// Remember a written link address.
    pub fn used_link(&mut self, href: &str) {
        if !self.links.iter().any(|l| l == href) {
            self.links.push(href.to_string());
        }
    }
    /// Remember a written media address.
    pub fn used_media(&mut self, src: &str) {
        if !self.media.iter().any(|l| l == src) {
            self.media.push(src.to_string());
        }
    }
}
