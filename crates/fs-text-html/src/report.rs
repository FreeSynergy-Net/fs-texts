//! What the export could not do exactly, as data.
//!
//! Pattern: Value object — plain data the caller (gate, site report) reads; nothing in here
//! decides.

/// One thing the export noticed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// An image has no alternative text.
    MissingAlt {
        /// The image address as written.
        src: String,
    },
    /// A link whose target the resolver does not know (written as plain text).
    DeadLink {
        /// The address as written.
        path: String,
    },
    /// A link whose address is not allowed (`javascript:` …), written as plain text.
    UnsafeLink {
        /// The address as written.
        path: String,
    },
    /// An image the media resolver refused.
    UnsafeMedia {
        /// The address as written.
        src: String,
    },
    /// Raw HTML that was not let through (written as text, or an attribute dropped).
    RawHtml {
        /// The part that was not let through.
        part: String,
    },
    /// A variable placeholder that was never filled in (nothing is written).
    UnfilledVariable {
        /// Its name.
        name: String,
    },
    /// A footnote reference without a definition.
    FootnoteUndefined {
        /// The label.
        label: String,
    },
    /// A footnote definition nobody refers to (left out).
    FootnoteUnused {
        /// The label.
        label: String,
    },
    /// More than one level-1 heading on a page.
    ManyTopHeadings,
    /// A heading level skipped (`h2` straight to `h4`).
    HeadingJump {
        /// Level before.
        from: u8,
        /// Level after.
        to: u8,
    },
}

/// The problems of one page, in the order they were found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The problems.
    pub problems: Vec<Problem>,
}

impl Report {
    /// Note a problem (the same one twice is noted once).
    pub fn note(&mut self, p: Problem) {
        if !self.problems.contains(&p) {
            self.problems.push(p);
        }
    }
    /// Nothing noticed?
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.problems.is_empty()
    }
}
