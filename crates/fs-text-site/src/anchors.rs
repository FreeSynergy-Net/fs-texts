//! Links to an anchor of a page of the site: the anchor must exist in the target page.
//!
//! Pattern: Specification over the rendered pages. A link `…/page/#name` (or `#name` on the page
//! itself) is dead if the target page has no element with `id="name"`. Links to other hosts are
//! not checked.

use crate::index::{page_url, SiteIndex};
use crate::prepare::Rendered;
use crate::report::{BuildReport, Gate};
use std::collections::{BTreeMap, BTreeSet};

/// All values of `id="…"` in a piece of HTML.
fn ids(html: &str) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    let mut rest = html;
    while let Some(i) = rest.find(" id=\"") {
        rest = &rest[i + 5..];
        if let Some(e) = rest.find('"') {
            out.insert(&rest[..e]);
        }
    }
    out
}

/// Report every link to a missing anchor.
pub(crate) fn check_anchors(
    rendered: &[Rendered],
    index: &SiteIndex,
    gate: Gate,
    rep: &mut BuildReport,
) {
    let by_url: BTreeMap<String, BTreeSet<&str>> = rendered
        .iter()
        .map(|r| {
            (
                page_url(&index.prefix, &r.entry.lang, &r.entry.path),
                // the ids of the page frame are anchors of every page
                ids(&r.html.body)
                    .into_iter()
                    .chain([fs_text_html::FRAME_MAIN_ID])
                    .collect(),
            )
        })
        .collect();
    for r in rendered {
        let own = page_url(&index.prefix, &r.entry.lang, &r.entry.path);
        for l in &r.html.links {
            let Some((page, frag)) = l.split_once('#') else {
                continue;
            };
            if frag.is_empty() {
                continue;
            }
            let target = if page.is_empty() { own.as_str() } else { page };
            // only pages of this site are known; a link to a folder page or elsewhere is not judged
            let Some(found) = by_url.get(target) else {
                continue;
            };
            if !found.contains(frag) {
                rep.dead_link(
                    &r.entry.origin,
                    format!("link {l:?}: the page has no anchor {frag:?}"),
                    gate,
                );
            }
        }
    }
}
