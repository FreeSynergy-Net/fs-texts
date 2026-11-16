//! Raw HTML in the text: let a short fixed list through, write the rest as text.
//!
//! Pattern: Strategy — [`RawHtmlPolicy`] (a closed set, one `match`) decides what happens to a
//! tag; the [`Cleaner`] scans, keeps the open tags balanced and reports what it did not let
//! through. Safe by construction: whatever is not an allowed tag becomes escaped text.

use crate::escape;
use crate::rawtag::{entity_len, keep_entities, parse_tag, Tag};
use crate::report::{Problem, Report};

/// What happens to raw HTML of the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RawHtmlPolicy {
    /// Let `details summary sub sup kbd u mark br` through, with the attributes `lang` and
    /// `title` only; everything else is written as text and reported.
    #[default]
    AllowList,
    /// Write all raw HTML as text.
    EscapeAll,
}

const ALLOWED: [&str; 8] = ["details", "summary", "sub", "sup", "kbd", "u", "mark", "br"];

fn allowed(name: &str) -> Option<&'static str> {
    ALLOWED.iter().copied().find(|a| *a == name)
}

/// Scans raw HTML and writes the safe form; keeps the allowed tags it opened balanced.
#[derive(Debug)]
pub struct Cleaner {
    policy: RawHtmlPolicy,
    open: Vec<&'static str>,
    floor: usize,
}

impl Cleaner {
    /// A cleaner with nothing open.
    #[must_use]
    pub fn new(policy: RawHtmlPolicy) -> Self {
        Self {
            policy,
            open: Vec::new(),
            floor: 0,
        }
    }
    /// Number of allowed tags open now.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.open.len()
    }
    /// The floor now.
    #[must_use]
    pub fn floor(&self) -> usize {
        self.floor
    }
    /// A closing tag may not close anything below `floor` (the surrounding mark's start).
    pub fn set_floor(&mut self, floor: usize) {
        self.floor = floor;
    }
    /// Close open tags down to `depth`.
    pub fn close_to(&mut self, depth: usize, out: &mut String) {
        while self.open.len() > depth {
            let name = self.open.pop().unwrap_or("");
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
    }

    /// Write the safe form of `raw` to `out`.
    pub fn feed(&mut self, raw: &str, out: &mut String, report: &mut Report) {
        let mut i = 0;
        while i < raw.len() {
            let rest = &raw[i..];
            if rest.starts_with("<!--") {
                if let Some(end) = rest.find("-->") {
                    i += end + 3;
                    continue;
                }
            }
            if rest.starts_with('<') {
                if let Some(tag) = parse_tag(rest) {
                    self.tag(&tag, &rest[..tag.len], out, report);
                    i += tag.len;
                    continue;
                }
            }
            let next = rest[1..].find(['<', '&']).map_or(rest.len(), |p| p + 1);
            let (chunk, adv) = if rest.starts_with('&') {
                let n = entity_len(rest);
                (&rest[..n.max(1)], n.max(1))
            } else {
                (&rest[..next], next)
            };
            if rest.starts_with('&') && entity_len(rest) > 0 {
                out.push_str(chunk);
            } else {
                escape::text(chunk, out);
            }
            i += adv;
        }
    }

    fn tag(&mut self, tag: &Tag, src: &str, out: &mut String, report: &mut Report) {
        let name = allowed(&tag.name).filter(|_| self.policy == RawHtmlPolicy::AllowList);
        let Some(name) = name else {
            return Self::as_text(src, out, report);
        };
        if tag.closing {
            match self.open.iter().rposition(|n| *n == name) {
                Some(at) if at >= self.floor => {
                    self.close_to(at + 1, out);
                    self.close_to(at, out);
                }
                _ => Self::as_text(src, out, report),
            }
            return;
        }
        out.push('<');
        out.push_str(name);
        let mut seen: Vec<&str> = Vec::new();
        for (k, v) in &tag.attrs {
            let ok = match k.as_str() {
                "lang" => {
                    !v.is_empty()
                        && v.len() <= 35
                        && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                }
                "title" => true,
                _ => false,
            };
            if ok && !seen.contains(&k.as_str()) {
                seen.push(k);
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                keep_entities(v, out);
                out.push('"');
            } else {
                report.note(Problem::RawHtml { part: k.clone() });
            }
        }
        out.push('>');
        if name != "br" {
            self.open.push(name);
        }
    }

    fn as_text(src: &str, out: &mut String, report: &mut Report) {
        escape::text(src, out);
        report.note(Problem::RawHtml {
            part: src.to_string(),
        });
    }
}
