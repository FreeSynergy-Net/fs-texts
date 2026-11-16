//! Task boxes and the footnote list of a page.
//!
//! Pattern: Template method (part of `Renderer`) — the pieces that need the whole page: the
//! footnotes come last, in the order of first reference, after everything that can refer to them.

use crate::escape;
use crate::render::Renderer;
use crate::tree::Node;

impl<'n> Renderer<'_, 'n> {
    /// The box of a task item; a first paragraph becomes its label. Returns the rest.
    pub(crate) fn task_head(
        &mut self,
        done: bool,
        kids: &'n [Node<'n>],
        tight: bool,
        out: &mut String,
    ) -> &'n [Node<'n>] {
        let check = if done { " checked" } else { "" };
        let word = if done {
            self.env.words.task_done
        } else {
            self.env.words.task_open
        };
        if let Some(Node::Leaf(b)) = kids
            .first()
            .filter(|n| matches!(n, Node::Leaf(b) if b.attrs.kind == fs_text::BlockKind::Paragraph))
        {
            if !tight {
                out.push_str("<p>");
            }
            out.push_str(&format!(
                "<label><input type=\"checkbox\" disabled{check}> "
            ));
            self.inline_of(b, out);
            out.push_str("</label>");
            if !tight {
                out.push_str("</p>");
            }
            out.push('\n');
            return &kids[1..];
        }
        out.push_str(&format!(
            "<input type=\"checkbox\" disabled{check} aria-label=\""
        ));
        escape::attr(word, out);
        out.push_str("\">");
        kids
    }

    pub(crate) fn footnotes(&mut self, out: &mut String) {
        let mut n = 0;
        let mut items = String::new();
        while n < self.st.notes.len() {
            let label = self.st.notes[n].clone();
            let def = self
                .stash
                .iter()
                .find(|(l, _)| *l == label)
                .map(|(_, k)| *k);
            n += 1;
            let mut inner = String::new();
            if let Some(kids) = def {
                self.nodes(kids, true, &mut inner);
            }
            let back = escape::attr_string(self.env.words.footnote_back);
            items.push_str(&format!(
                "<li id=\"fn-{n}\">{} <a class=\"fn-back\" href=\"#fnref-{n}\" aria-label=\"{back}\">\u{21a9}</a></li>\n",
                inner.trim_end()
            ));
        }
        for (l, _) in &self.stash {
            if !self.st.notes.iter().any(|x| x == l) {
                self.st.report.note(crate::Problem::FootnoteUnused {
                    label: (*l).to_string(),
                });
            }
        }
        if !items.is_empty() {
            let h = escape::text_string(self.env.words.footnotes);
            out.push_str(&format!(
                "<section class=\"footnotes\" aria-label=\"{h}\">\n<hr>\n<ol>\n{items}</ol>\n</section>\n"
            ));
        }
    }
}

pub(crate) fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| {
        f.to_ascii_uppercase().to_string() + c.as_str()
    })
}
