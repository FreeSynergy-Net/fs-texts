//! Heading ids: a fixed ASCII rule, unique within one page.
//!
//! Pattern: Value object (the id set of one page) + pure function `slug`.

use std::collections::BTreeSet;

/// Ids the page frame uses itself; a heading never takes them.
const RESERVED: [&str; 6] = ["main", "content", "nav", "skip", "top", "footer"];

/// The ASCII id of a heading text: letters and digits lower-cased, `ä ö ü ß` as `ae oe ue ss`,
/// runs of space, `-` and `_` as one `-`, everything else dropped. Empty result is `section`.
/// A result that would clash with a footnote id (`fn-…`, `fnref-…`) gets the prefix `h-`.
#[must_use]
pub fn slug(text: &str) -> String {
    let mut s = String::new();
    let mut gap = false;
    for c in text.chars() {
        let piece: Option<&str> = match c {
            'ä' | 'Ä' => Some("ae"),
            'ö' | 'Ö' => Some("oe"),
            'ü' | 'Ü' => Some("ue"),
            'ß' => Some("ss"),
            _ => None,
        };
        if let Some(p) = piece {
            push_gap(&mut s, &mut gap);
            s.push_str(p);
        } else if c.is_ascii_alphanumeric() {
            push_gap(&mut s, &mut gap);
            s.push(c.to_ascii_lowercase());
        } else if c.is_whitespace() || c == '-' || c == '_' {
            gap = true;
        }
    }
    if s.is_empty() {
        s.push_str("section");
    }
    if s.starts_with("fn-") || s.starts_with("fnref-") {
        s.insert_str(0, "h-");
    }
    s
}

fn push_gap(s: &mut String, gap: &mut bool) {
    if *gap && !s.is_empty() {
        s.push('-');
    }
    *gap = false;
}

/// The ids already used on one page.
#[derive(Debug)]
pub struct Ids {
    used: BTreeSet<String>,
}

impl Default for Ids {
    fn default() -> Self {
        Self {
            used: RESERVED.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

impl Ids {
    /// Take a free id for `base`: the slug, then `slug-2`, `slug-3`, ….
    pub fn unique(&mut self, base: &str) -> String {
        self.take(slug(base))
    }

    /// Take `wanted` itself, or the next free `wanted-2`, ….
    pub fn take(&mut self, wanted: String) -> String {
        let mut id = wanted.clone();
        let mut n = 2;
        while !self.used.insert(id.clone()) {
            id = format!("{wanted}-{n}");
            n += 1;
        }
        id
    }
}
