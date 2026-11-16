//! The last steps of phase 2: the files of the site that the user did not bring.
//!
//! Pattern: the same Builder steps as `write`, in a third file (one context, several parts).

use crate::config::Lang;
use crate::error::SiteError;
use crate::gen;
use crate::outfs::{Kind, Out};
use crate::words::SiteWords;
use crate::write::Writer;
use fs_text_html::pass::MediaType;
use fs_text_html::{render_page, HtmlPage, Navigation};

impl Writer<'_> {
    /// Language choice, 404, sitemap, robots, `site.toml`: what the user did not bring.
    pub(crate) fn site_files(&mut self, starts: &[(Lang, String)]) -> Result<(), SiteError> {
        let cfg = self.cfg;
        let dl = &cfg.default_language;
        let mut plain = self.frame(dl, Navigation::default());
        plain.canonical = Some(format!("{}/", self.origin));
        if !self.supplied.contains("index.html") {
            let page = HtmlPage {
                body: gen::choose_body(cfg, starts),
                title: Some(cfg.name.clone()),
                ..HtmlPage::default()
            };
            self.out.put(
                "index.html",
                render_page(&plain, &page).as_bytes(),
                "text/html",
                Kind::Generated,
            )?;
        }
        if !self.supplied.contains("404.html") {
            let page = HtmlPage {
                body: gen::not_found_body(cfg),
                title: Some(SiteWords::for_lang(dl.as_str()).not_found.to_string()),
                ..HtmlPage::default()
            };
            self.out.put(
                "404.html",
                render_page(&plain, &page).as_bytes(),
                "text/html",
                Kind::Generated,
            )?;
        }
        self.out.put(
            "sitemap.xml",
            gen::sitemap(cfg, &self.by_path).as_bytes(),
            MediaType::Xml.mime(),
            Kind::Generated,
        )?;
        if cfg.path_prefix.is_root() && !self.supplied.contains("robots.txt") {
            self.out.put(
                "robots.txt",
                gen::robots(cfg).as_bytes(),
                "text/plain",
                Kind::Generated,
            )?;
        }
        self.out.put(
            "site.toml",
            cfg.to_schema1().as_bytes(),
            "text/x-toml",
            Kind::Generated,
        )
    }
}

pub(crate) fn put_page(
    out: &mut Out,
    lang: &Lang,
    path: &[String],
    html: &str,
) -> Result<(), SiteError> {
    let mut p = format!("{}/", lang.as_str());
    for s in path {
        p.push_str(s);
        p.push('/');
    }
    p.push_str("index.html");
    out.put(&p, html.as_bytes(), "text/html", Kind::Page)
}
