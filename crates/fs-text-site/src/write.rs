//! Phase 2 of the build: write the output folder.
//!
//! Pattern: Builder steps over one context ([`Writer`]): user files, style sheet, media, one
//! pass per language, the files of the site, the manifest. Every file is new; every copy is
//! compared with the hash measured in phase 1.

use crate::build::BuildOptions;
use crate::config::{Lang, SiteConfig};
use crate::error::SiteError;
use crate::gen;
use crate::index::{page_url, SiteIndex};
use crate::outfs::{hex, Kind, Out};
use crate::prepare::Rendered;
use crate::report::BuildReport;
use crate::source::{FileEntry, SiteSource};
use crate::tree::{build as build_tree, render_full, TreePage};
use crate::words::SiteWords;
use fs_text_html::pass::{Limits, MediaType};
use fs_text_html::{render_page, Alternate, Csp, Frame, HtmlPage, NavNode, Navigation};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) fn problem(e: impl std::fmt::Display) -> SiteError {
    SiteError::Problems(vec![e.to_string()])
}

/// Everything the write steps share.
pub(crate) struct Writer<'a> {
    pub(crate) src: &'a dyn SiteSource,
    pub(crate) cfg: &'a SiteConfig,
    pub(crate) out: &'a mut Out,
    pub(crate) index: &'a Arc<SiteIndex>,
    pub(crate) limits: Limits,
    pub(crate) rendered: &'a [Rendered],
    pub(crate) prefix: String,
    pub(crate) origin: String,
    pub(crate) css_url: String,
    pub(crate) csp: Csp,
    pub(crate) by_path: BTreeMap<Vec<String>, Vec<Lang>>,
    pub(crate) supplied: BTreeSet<String>,
}

impl<'a> Writer<'a> {
    pub(crate) fn new(
        src: &'a dyn SiteSource,
        out: &'a mut Out,
        index: &'a Arc<SiteIndex>,
        limits: Limits,
        rendered: &'a [Rendered],
    ) -> Self {
        let cfg = src.config();
        let mut by_path: BTreeMap<Vec<String>, Vec<Lang>> = BTreeMap::new();
        for r in rendered {
            by_path
                .entry(r.entry.path.clone())
                .or_default()
                .push(r.entry.lang.clone());
        }
        Self {
            src,
            cfg,
            out,
            index,
            limits,
            rendered,
            prefix: cfg.url_prefix(),
            origin: cfg.origin(),
            css_url: String::new(),
            csp: cfg
                .csp_policy
                .as_ref()
                .map_or(Csp::Strict, |p| Csp::Custom(p.clone())),
            by_path,
            supplied: BTreeSet::new(),
        }
    }

    /// All steps.
    pub(crate) fn run(
        mut self,
        passed: &[(FileEntry, [u8; 32], bool)],
        opt: &BuildOptions,
        rep: &mut BuildReport,
    ) -> Result<(), SiteError> {
        self.copy_pass(passed)?;
        self.style_sheet()?;
        self.copy_media(rep)?;
        let mut starts: Vec<(Lang, String)> = Vec::new();
        for lang in &self.cfg.languages {
            if let Some(start) = self.language(lang, rep)? {
                starts.push((lang.clone(), start));
            }
        }
        self.site_files(&starts)?;
        let m = gen::manifest(self.cfg, self.src.revision(), &self.out.ledger);
        rep.files = self.out.ledger.len();
        rep.manifest_sha256 = hex(&Sha256::digest(m.as_bytes()));
        self.out.put(
            "site-manifest.json",
            m.as_bytes(),
            "application/json",
            Kind::Generated,
        )?;
        self.out.finish(opt.mtime)
    }

    fn abs(&self, lang: &Lang, path: &[String]) -> String {
        format!(
            "https://{}{}",
            self.cfg.host.as_str(),
            page_url(&self.prefix, lang, path)
        )
    }

    fn frame(&self, lang: &Lang, nav: Navigation) -> Frame {
        Frame {
            lang: lang.as_str().to_string(),
            site_name: self.cfg.name.clone(),
            canonical: None,
            alternates: Vec::new(),
            x_default: Some(format!("{}/", self.origin)),
            stylesheet: self.css_url.clone(),
            csp: self.csp.clone(),
            nav,
            current: String::new(),
            footer: None,
        }
    }

    fn title_of(&self, r: &Rendered) -> String {
        r.html
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| {
                r.entry
                    .path
                    .last()
                    .cloned()
                    .unwrap_or_else(|| self.cfg.name.clone())
            })
    }

    /// Pages and contents page of one language. Returns the address its start leads to.
    fn language(
        &mut self,
        lang: &Lang,
        rep: &mut BuildReport,
    ) -> Result<Option<String>, SiteError> {
        let rendered = self.rendered;
        let mine: Vec<&Rendered> = rendered.iter().filter(|r| &r.entry.lang == lang).collect();
        if mine.is_empty() {
            rep.note(
                "site.toml",
                format!("language {} has no page", lang.as_str()),
                false,
            );
            return Ok(None);
        }
        let w = SiteWords::for_lang(lang.as_str());
        let tree_pages: Vec<TreePage> = mine
            .iter()
            .map(|r| TreePage {
                path: r.entry.path.clone(),
                title: self.title_of(r),
                order: r.entry.order,
            })
            .collect();
        let contents_path = vec![w.contents_slug.to_string()];
        let contents_url = page_url(&self.prefix, lang, &contents_path);
        let node = NavNode {
            title: w.contents.to_string(),
            href: contents_url.clone(),
            children: Vec::new(),
        };
        let core = build_tree(&self.prefix, lang, &tree_pages, Vec::new());
        let mut frame = self.frame(
            lang,
            build_tree(&self.prefix, lang, &tree_pages, vec![node]),
        );
        for r in &mine {
            let mut page = r.html.clone();
            page.title = Some(self.title_of(r));
            frame.current = page_url(&self.prefix, lang, &r.entry.path);
            frame.canonical = Some(self.abs(lang, &r.entry.path));
            frame.alternates = self.by_path[&r.entry.path]
                .iter()
                .map(|l| Alternate {
                    lang: l.as_str().to_string(),
                    href: self.abs(l, &r.entry.path),
                })
                .collect();
            put_page(self.out, lang, &r.entry.path, &render_page(&frame, &page))?;
        }
        let page = HtmlPage {
            body: render_full(&core),
            title: Some(w.contents.to_string()),
            ..HtmlPage::default()
        };
        let contents_at = |l: &Lang| {
            self.abs(
                l,
                &[SiteWords::for_lang(l.as_str()).contents_slug.to_string()],
            )
        };
        frame.current.clone_from(&contents_url);
        frame.canonical = Some(contents_at(lang));
        frame.alternates = self
            .cfg
            .languages
            .iter()
            .filter(|l| rendered.iter().any(|r| &r.entry.lang == *l))
            .map(|l| Alternate {
                lang: l.as_str().to_string(),
                href: contents_at(l),
            })
            .collect();
        put_page(self.out, lang, &contents_path, &render_page(&frame, &page))?;
        rep.pages += mine.len() + 1;
        if mine.iter().any(|r| r.entry.path.is_empty()) {
            Ok(Some(page_url(&self.prefix, lang, &[])))
        } else {
            rep.note(
                &format!("content/{}", lang.as_str()),
                "no start page (index.md)",
                false,
            );
            Ok(Some(contents_url))
        }
    }

    /// Language choice, 404, sitemap, robots, `site.toml`: what the user did not bring.
    fn site_files(&mut self, starts: &[(Lang, String)]) -> Result<(), SiteError> {
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

fn put_page(out: &mut Out, lang: &Lang, path: &[String], html: &str) -> Result<(), SiteError> {
    let mut p = format!("{}/", lang.as_str());
    for s in path {
        p.push_str(s);
        p.push('/');
    }
    p.push_str("index.html");
    out.put(&p, html.as_bytes(), "text/html", Kind::Page)
}
