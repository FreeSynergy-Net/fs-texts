//! Leaf blocks: paragraph, heading, code, HTML, rule, table, front matter, raw text.
//!
//! Pattern: Visitor — one `match` over the block kind, outside the data.

use crate::escape;
use crate::page::{front_matter, TocEntry};
use crate::render::Renderer;
use crate::{inline, table, Problem};
use fs_text::markdown::HARD_BREAK;
use fs_text::{Block, BlockKind};

impl Renderer<'_, '_> {
    /// Write the text of `b` with its marks.
    pub(crate) fn inline_of(&mut self, b: &Block, out: &mut String) {
        let chars: Vec<char> = b.text.chars().collect();
        inline::render(
            self.env,
            &mut self.st,
            &chars,
            b.marks.as_slice(),
            (0, chars.len()),
            out,
        );
    }

    pub(crate) fn leaf(&mut self, b: &Block, tight: bool, out: &mut String) {
        match &b.attrs.kind {
            BlockKind::Paragraph => {
                if tight {
                    self.inline_of(b, out);
                } else {
                    out.push_str("<p>");
                    self.inline_of(b, out);
                    out.push_str("</p>");
                }
                out.push('\n');
            }
            BlockKind::Heading { level } => self.heading(b, (*level).clamp(1, 6), out),
            BlockKind::Code { info } => {
                out.push_str("<pre><code");
                let lang: String = info
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || "_+#.-".contains(*c))
                    .collect();
                if !lang.is_empty() {
                    out.push_str(&format!(" class=\"language-{lang}\""));
                }
                out.push('>');
                escape::text(&b.text.to_string(), out);
                if !b.text.to_string().ends_with('\n') {
                    out.push('\n');
                }
                out.push_str("</code></pre>\n");
            }
            // a link reference definition: Markdown source the parser used, not HTML
            BlockKind::Html if b.attrs.extra.contains_key("unparsed") => {}
            BlockKind::Html => {
                let before = self.raw.depth();
                let mut inner = String::new();
                self.raw
                    .feed(&b.text.to_string(), &mut inner, &mut self.st.report);
                // a tag left open (or opened earlier) spans blocks: no wrapper then
                if before > 0 || self.raw.depth() > 0 {
                    out.push_str(inner.trim_matches('\n'));
                    out.push('\n');
                } else {
                    out.push_str("<div class=\"raw\">");
                    out.push_str(inner.trim_matches('\n'));
                    out.push_str("</div>\n");
                }
            }
            BlockKind::ThematicBreak => out.push_str("<hr>\n"),
            BlockKind::Table { align } => table::render(self.env, &mut self.st, b, align, out),
            BlockKind::FrontMatter => self.meta = front_matter(&b.text.to_string()),
            BlockKind::Raw => {
                out.push_str("<pre>");
                escape::text(&b.text.to_string(), out);
                out.push_str("</pre>\n");
            }
        }
    }

    fn heading(&mut self, b: &Block, level: u8, out: &mut String) {
        let plain: String = b
            .text
            .chars()
            .map(|c| if c == HARD_BREAK { ' ' } else { c })
            .collect();
        let id = match own_id(b) {
            Some(w) => self.st.ids.take(&w),
            None => self.st.ids.unique(&plain),
        };
        if level == 1 {
            self.tops += 1;
        }
        if self.last_level != 0 && level > self.last_level + 1 {
            self.st.report.note(Problem::HeadingJump {
                from: self.last_level,
                to: level,
            });
        }
        self.last_level = level;
        out.push_str(&format!("<h{level} id=\""));
        escape::attr(&id, out);
        out.push_str("\">");
        self.inline_of(b, out);
        out.push_str(&format!("</h{level}>\n"));
        self.toc.push(TocEntry {
            level,
            id,
            text: plain.trim().to_string(),
        });
    }
}

/// The `{#id}` of a heading, if it is a plain id.
fn own_id(b: &Block) -> Option<String> {
    let a = b.attrs.heading_attrs.as_deref()?;
    let id = a
        .trim_matches(['{', '}'])
        .split_whitespace()
        .find_map(|w| w.strip_prefix('#'))?;
    let ok = id.starts_with(|c: char| c.is_ascii_alphabetic())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    ok.then(|| id.to_string())
}
