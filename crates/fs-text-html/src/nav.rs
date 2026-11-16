//! The short navigation of one page.
//!
//! Pattern: Composite (the tree) + Visitor (one walk that writes only what a reader needs).
//! A page shows the top level, the path to itself (each ancestor with only the next step of
//! the path under it), its siblings, and its children; never the whole tree. The full tree
//! belongs on a contents page of its own.

use crate::escape;

/// Most entries of one list in the short navigation (the page on the path is always shown).
pub const MAX_SHOWN: usize = 40;

/// One entry of the navigation tree; the caller orders the children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavNode {
    /// Title shown.
    pub title: String,
    /// Address of the page (as written into `href`).
    pub href: String,
    /// Pages below.
    pub children: Vec<NavNode>,
}

/// The navigation tree of a site (one language).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Navigation {
    /// Top level.
    pub roots: Vec<NavNode>,
}

impl Navigation {
    /// Indices from the top level down to the page with address `href`.
    #[must_use]
    pub fn path_to(&self, href: &str) -> Vec<usize> {
        fn walk(list: &[NavNode], href: &str, path: &mut Vec<usize>) -> bool {
            for (i, n) in list.iter().enumerate() {
                path.push(i);
                if n.href == href || walk(&n.children, href, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        let mut p = Vec::new();
        if walk(&self.roots, href, &mut p) {
            p
        } else {
            Vec::new()
        }
    }

    /// `(title, href)` from the top level down to the page itself (empty if unknown).
    #[must_use]
    pub fn trail(&self, href: &str) -> Vec<(&str, &str)> {
        let mut list = &self.roots;
        let mut out = Vec::new();
        for i in self.path_to(href) {
            out.push((list[i].title.as_str(), list[i].href.as_str()));
            list = &list[i].children;
        }
        out
    }

    /// The `<ul>` of the short navigation for the page `href`.
    #[must_use]
    pub fn render(&self, href: &str) -> String {
        let path = self.path_to(href);
        let mut out = String::new();
        level(&self.roots, &path, 0, href, &mut out);
        out
    }
}

fn level(list: &[NavNode], path: &[usize], depth: usize, current: &str, out: &mut String) {
    if list.is_empty() {
        return;
    }
    let on_path = depth < path.len();
    let all = depth == 0 || !on_path || depth + 1 == path.len();
    out.push_str("<ul>\n");
    let mut shown = 0;
    for (i, n) in list.iter().enumerate() {
        let mine = on_path && path[depth] == i;
        if !all && !mine {
            continue;
        }
        // The budget is kept in code, not only in a test: a flat folder of thousands of pages
        // shows its first entries and the page itself; the contents page lists the rest.
        if !mine && shown >= MAX_SHOWN {
            continue;
        }
        shown += 1;
        out.push_str("<li><a href=\"");
        escape::attr(&n.href, out);
        out.push('"');
        if n.href == current {
            out.push_str(" aria-current=\"page\"");
        }
        out.push('>');
        escape::text(&n.title, out);
        out.push_str("</a>");
        if mine {
            out.push('\n');
            level(&n.children, path, depth + 1, current, out);
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n");
}
