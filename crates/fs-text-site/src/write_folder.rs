//! Overview pages of folders without an index page.
//!
//! Pattern: the same Builder steps as `write`, in a fourth file. Nothing is invented: the page
//! lists what lies below the folder, with the titles of the pages, and says it was generated.

use crate::config::Lang;
use crate::error::SiteError;
use crate::index::page_url;
use crate::tree::{find, render_full};
use crate::words::SiteWords;
use crate::write::Writer;
use crate::write_site::put_page;
use fs_text_html::escape::text_string;
use fs_text_html::{render_page, Alternate, Frame, HtmlPage, Navigation};
use std::collections::BTreeSet;

impl Writer<'_> {
    /// The overview pages of `folders` in `lang`; `core` is the tree without the extra links.
    pub(crate) fn folder_pages(
        &mut self,
        lang: &Lang,
        folders: &BTreeSet<Vec<String>>,
        core: &Navigation,
        frame: &mut Frame,
    ) -> Result<(), SiteError> {
        let w = SiteWords::for_lang(lang.as_str());
        for f in folders {
            let url = page_url(&self.prefix, lang, f);
            let kids = find(&core.roots, &url).map(|n| n.children.clone());
            let list = Navigation {
                roots: kids
                    .unwrap_or_default()
                    .into_iter()
                    .map(|mut c| {
                        c.children.clear();
                        c
                    })
                    .collect(),
                ..Navigation::default()
            };
            let name = f.last().cloned().unwrap_or_default();
            let page = HtmlPage {
                body: format!(
                    "<p class=\"auto\"><em>{}</em></p>\n{}",
                    text_string(w.folder_auto),
                    render_full(&list)
                ),
                title: Some(name),
                ..HtmlPage::default()
            };
            frame.current.clone_from(&url);
            frame.canonical = Some(self.abs(lang, f));
            frame.alternates = self.by_path[f]
                .iter()
                .map(|l| Alternate {
                    lang: l.as_str().to_string(),
                    href: self.abs(l, f),
                })
                .collect();
            frame.csp = self.csp_for(f);
            put_page(self.out, lang, f, &render_page(frame, &page))?;
        }
        Ok(())
    }
}
