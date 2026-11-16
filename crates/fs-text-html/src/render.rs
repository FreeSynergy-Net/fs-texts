//! A block sequence as an HTML fragment.
//!
//! Pattern: Template method over a Visitor — `Renderer` walks the container tree
//! (`tree.rs`), writes lists, quotes, callouts and footnotes here and hands leaf blocks to
//! `leaf.rs`; one `match` per container kind.

use crate::escape;
use crate::page::{HtmlPage, TocEntry};
use crate::raw::Cleaner;
use crate::state::{Env, PageState};
use crate::tree::{build, footnote_labels, Node};
use fs_text::{Block, Container};

pub(crate) struct Renderer<'e, 'n> {
    pub env: &'e Env<'e>,
    pub st: PageState,
    pub toc: Vec<TocEntry>,
    pub last_level: u8,
    pub tops: usize,
    pub meta: (Option<String>, Option<String>, Option<String>),
    stash: Vec<(&'n str, &'n [Node<'n>])>,
    /// Raw HTML tags open across the HTML blocks of one sequence (`<details>` … `</details>`).
    pub raw: Cleaner,
}

/// Render `blocks` into a page.
pub(crate) fn render_blocks(env: &Env, blocks: &[Block]) -> HtmlPage {
    let tree = build(blocks);
    let mut r = Renderer {
        env,
        st: PageState::default(),
        toc: Vec::new(),
        last_level: 0,
        tops: 0,
        meta: (None, None, None),
        stash: Vec::new(),
        raw: Cleaner::new(env.raw),
    };
    footnote_labels(&tree, &mut r.st.defined);
    let mut body = String::new();
    r.nodes(&tree, false, &mut body);
    r.footnotes(&mut body);
    let (mut title, description, lang) = std::mem::take(&mut r.meta);
    if title.is_none() {
        title = r.toc.iter().find(|t| t.level == 1).map(|t| t.text.clone());
    }
    if r.tops > 1 {
        r.st.report.note(crate::Problem::ManyTopHeadings);
    }
    HtmlPage {
        body,
        title,
        description,
        lang,
        toc: r.toc,
        top_headings: r.tops,
        links: r.st.links,
        media: r.st.media,
        report: r.st.report,
    }
}

impl<'n> Renderer<'_, 'n> {
    pub(crate) fn nodes(&mut self, nodes: &'n [Node<'n>], tight: bool, out: &mut String) {
        let (base, floor) = (self.raw.depth(), self.raw.floor());
        self.raw.set_floor(base);
        self.sequence(nodes, tight, out);
        self.raw.close_to(base, out);
        self.raw.set_floor(floor);
    }

    fn sequence(&mut self, nodes: &'n [Node<'n>], tight: bool, out: &mut String) {
        let mut i = 0;
        while i < nodes.len() {
            match &nodes[i] {
                Node::Leaf(b) => self.leaf(b, tight, out),
                Node::Group(Container::Item { list, number, .. }, _) => {
                    let mut j = i + 1;
                    while let Some(Node::Group(Container::Item { list: l, .. }, _)) = nodes.get(j) {
                        if l != list {
                            break;
                        }
                        j += 1;
                    }
                    self.list(&nodes[i..j], *number, out);
                    i = j;
                    continue;
                }
                Node::Group(c, kids) => self.group(c, kids, out),
            }
            i += 1;
        }
    }

    fn group(&mut self, c: &'n Container, kids: &'n [Node<'n>], out: &mut String) {
        match c {
            Container::Quote { .. } => {
                out.push_str("<blockquote>\n");
                self.nodes(kids, false, out);
                out.push_str("</blockquote>\n");
            }
            Container::Callout { ty, title, fold, .. } => {
                let ty = escape::class_part(ty);
                let ty = if ty.is_empty() { "note".to_string() } else { ty };
                let title = if title.trim().is_empty() { capital(&ty) } else { title.trim().to_string() };
                let t = escape::text_string(&title);
                if let Some(f) = fold {
                    let open = if *f == '+' { " open" } else { "" };
                    out.push_str(&format!(
                        "<details class=\"callout callout-{ty}\"{open}>\n<summary>{t}</summary>\n"
                    ));
                    self.nodes(kids, false, out);
                    out.push_str("</details>\n");
                } else {
                    out.push_str(&format!(
                        "<aside class=\"callout callout-{ty}\" role=\"note\">\n<p class=\"callout-title\">{t}</p>\n"
                    ));
                    self.nodes(kids, false, out);
                    out.push_str("</aside>\n");
                }
            }
            Container::Footnote { label, .. } => self.stash.push((label.as_str(), kids)),
            Container::Item { .. } => {}
        }
    }

    fn list(&mut self, items: &'n [Node<'n>], number: Option<u64>, out: &mut String) {
        match number {
            Some(1) => out.push_str("<ol>\n"),
            Some(n) => out.push_str(&format!("<ol start=\"{n}\">\n")),
            None => out.push_str("<ul>\n"),
        }
        for it in items {
            let Node::Group(Container::Item { tight, task, .. }, kids) = it else {
                continue;
            };
            out.push_str(if task.is_some() { "<li class=\"task\">" } else { "<li>" });
            let mut kids: &'n [Node<'n>] = kids;
            if let Some(done) = task {
                kids = self.task_head(*done, kids, *tight, out);
            }
            if (!kids.is_empty() && !matches!(kids[0], Node::Leaf(_))) || !*tight {
                out.push('\n');
            }
            self.nodes(kids, *tight, out);
            out.push_str("</li>\n");
        }
        out.push_str(if number.is_some() { "</ol>\n" } else { "</ul>\n" });
    }

    /// The box of a task item; a first paragraph becomes its label. Returns the rest.
    fn task_head(&mut self, done: bool, kids: &'n [Node<'n>], tight: bool, out: &mut String) -> &'n [Node<'n>] {
        let check = if done { " checked" } else { "" };
        let word = if done { self.env.words.task_done } else { self.env.words.task_open };
        if let Some(Node::Leaf(b)) = kids.first().filter(|n| matches!(n, Node::Leaf(b) if b.attrs.kind == fs_text::BlockKind::Paragraph)) {
            if !tight {
                out.push_str("<p>");
            }
            out.push_str(&format!("<label><input type=\"checkbox\" disabled{check}> "));
            self.inline_of(b, out);
            out.push_str("</label>");
            if !tight {
                out.push_str("</p>");
            }
            out.push('\n');
            return &kids[1..];
        }
        out.push_str(&format!("<input type=\"checkbox\" disabled{check} aria-label=\""));
        escape::attr(word, out);
        out.push_str("\">");
        kids
    }

    fn footnotes(&mut self, out: &mut String) {
        let mut n = 0;
        let mut items = String::new();
        while n < self.st.notes.len() {
            let label = self.st.notes[n].clone();
            let def = self.stash.iter().find(|(l, _)| *l == label).map(|(_, k)| *k);
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
                self.st.report.note(crate::Problem::FootnoteUnused { label: (*l).to_string() });
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

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_ascii_uppercase().to_string() + c.as_str())
}
