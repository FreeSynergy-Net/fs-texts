//! The navigation tree of one language, and its full rendering on the contents page.
//!
//! Pattern: Builder of a Composite ([`fs_text_html::NavNode`]). A page hangs under its nearest
//! existing ancestor; siblings are ordered by the `.pages` position, then naturally (numbers in
//! names count as numbers: `2` before `10`).

use crate::config::Lang;
use crate::index::page_url;
use fs_text_html::escape;
use fs_text_html::{NavNode, Navigation};
use std::cmp::Ordering;
use std::collections::HashMap;

/// One page for the tree.
pub struct TreePage {
    /// Address segments.
    pub path: Vec<String>,
    /// Shown title.
    pub title: String,
    /// Position among siblings.
    pub order: u32,
}

/// Compare two names, numbers inside as numbers.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.as_bytes(), b.as_bytes());
    loop {
        match (x.first(), y.first()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let n = x.iter().take_while(|b| b.is_ascii_digit()).count();
                let m = y.iter().take_while(|b| b.is_ascii_digit()).count();
                let (dx, dy) = (trim_zeros(&x[..n]), trim_zeros(&y[..m]));
                let o = dx.len().cmp(&dy.len()).then_with(|| dx.cmp(dy));
                if o != Ordering::Equal {
                    return o;
                }
                x = &x[n..];
                y = &y[m..];
            }
            (Some(c), Some(d)) => {
                if c != d {
                    return c.cmp(d);
                }
                x = &x[1..];
                y = &y[1..];
            }
        }
    }
}

fn trim_zeros(d: &[u8]) -> &[u8] {
    let z = d
        .iter()
        .take_while(|b| **b == b'0')
        .count()
        .min(d.len().saturating_sub(1));
    &d[z..]
}

/// The navigation of one language. The `extra` links (contents page, sitemap) are kept apart: the
/// navigation shows them on every page, outside its budget; `more` is the label of the hint.
#[must_use]
pub fn build(
    prefix: &str,
    lang: &Lang,
    pages: &[TreePage],
    extra: Vec<NavNode>,
    more: &str,
) -> Navigation {
    let by_path: HashMap<&[String], usize> = pages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.path.as_slice(), i))
        .collect();
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); pages.len()];
    let mut top: Vec<usize> = Vec::new();
    for (i, p) in pages.iter().enumerate() {
        let parent = (1..p.path.len())
            .rev()
            .find_map(|n| by_path.get(&p.path[..n]).copied());
        match parent {
            Some(pi) if !p.path.is_empty() => kids[pi].push(i),
            _ => top.push(i),
        }
    }
    let cmp = |a: &usize, b: &usize| {
        let (pa, pb) = (&pages[*a], &pages[*b]);
        pa.path
            .is_empty()
            .cmp(&pb.path.is_empty())
            .reverse()
            .then(pa.order.cmp(&pb.order))
            .then_with(|| natural_cmp(&pa.path.join("/"), &pb.path.join("/")))
    };
    top.sort_by(cmp);
    for k in &mut kids {
        k.sort_by(cmp);
    }
    #[allow(clippy::items_after_statements)]
    fn node(
        i: usize,
        pages: &[TreePage],
        kids: &[Vec<usize>],
        prefix: &str,
        lang: &Lang,
    ) -> NavNode {
        NavNode {
            title: pages[i].title.clone(),
            href: page_url(prefix, lang, &pages[i].path),
            children: kids[i]
                .iter()
                .map(|&k| node(k, pages, kids, prefix, lang))
                .collect(),
        }
    }
    let roots: Vec<NavNode> = top
        .iter()
        .map(|&i| node(i, pages, &kids, prefix, lang))
        .collect();
    Navigation {
        roots,
        extra,
        more: more.to_string(),
    }
}

/// The full tree as nested lists, for the contents page.
#[must_use]
pub fn render_full(nav: &Navigation) -> String {
    fn list(nodes: &[NavNode], out: &mut String) {
        if nodes.is_empty() {
            return;
        }
        out.push_str("<ul>\n");
        for n in nodes {
            out.push_str("<li><a href=\"");
            escape::attr(&n.href, out);
            out.push_str("\">");
            escape::text(&n.title, out);
            out.push_str("</a>");
            if !n.children.is_empty() {
                out.push('\n');
                list(&n.children, out);
            }
            out.push_str("</li>\n");
        }
        out.push_str("</ul>\n");
    }
    let mut o = String::new();
    list(&nav.roots, &mut o);
    o
}

/// The entry with the address `href` somewhere in `nodes`.
#[must_use]
pub fn find<'a>(nodes: &'a [NavNode], href: &str) -> Option<&'a NavNode> {
    nodes.iter().find_map(|n| {
        if n.href == href {
            Some(n)
        } else {
            find(&n.children, href)
        }
    })
}
