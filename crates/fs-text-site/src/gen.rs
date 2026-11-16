//! The files the builder writes itself: sitemap, robots, language choice, 404, manifest.
//!
//! Pattern: Template method per file (a fixed text with the changing parts filled in). No time
//! stamps anywhere (determinism): the sitemap has no `lastmod`, the manifest no build time.

use crate::config::{Lang, SiteConfig};
use crate::index::page_url;
use crate::outfs::{hex, Entry};
use fs_text_html::escape::{attr_string, text_string};
use fs_text_html::words::autonym;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// `sitemap.xml`: every page once per language, each with all its variants and `x-default`.
#[must_use]
pub fn sitemap(cfg: &SiteConfig, pages: &BTreeMap<Vec<String>, Vec<Lang>>) -> String {
    let origin = cfg.origin();
    let prefix = cfg.url_prefix();
    let mut o = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    let abs = |l: &Lang, p: &[String]| {
        format!("https://{}{}", cfg.host.as_str(), page_url(&prefix, l, p))
    };
    for (path, langs) in pages {
        for l in langs {
            o.push_str(&format!(
                "<url>\n<loc>{}</loc>\n",
                text_string(&abs(l, path))
            ));
            // Same set as the HTML pages: all variants (also one) and x-default.
            for a in langs {
                o.push_str(&format!(
                    "<xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{}\"/>\n",
                    attr_string(a.as_str()),
                    attr_string(&abs(a, path))
                ));
            }
            o.push_str(&format!(
                "<xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{}/\"/>\n",
                attr_string(&origin)
            ));
            o.push_str("</url>\n");
        }
    }
    o.push_str("</urlset>\n");
    o
}

/// `robots.txt` (only for a site at the root of its host).
#[must_use]
pub fn robots(cfg: &SiteConfig) -> String {
    format!(
        "User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n",
        cfg.origin()
    )
}

/// The body of the language choice page: `(language, address of its start)`.
#[must_use]
pub fn choose_body(cfg: &SiteConfig, targets: &[(Lang, String)]) -> String {
    let w = crate::words::SiteWords::for_lang(cfg.default_language.as_str());
    let mut o = format!("<p>{}</p>\n<ul>\n", text_string(w.choose_language));
    for (l, href) in targets {
        o.push_str(&format!(
            "<li><a href=\"{}\" hreflang=\"{}\" lang=\"{}\">{}</a></li>\n",
            attr_string(href),
            attr_string(l.as_str()),
            attr_string(l.as_str()),
            text_string(autonym(l.as_str()))
        ));
    }
    o.push_str("</ul>\n");
    o
}

/// The body of the 404 page.
#[must_use]
pub fn not_found_body(cfg: &SiteConfig) -> String {
    let w = crate::words::SiteWords::for_lang(cfg.default_language.as_str());
    format!(
        "<p>{}</p>\n<p><a href=\"{}\">{}</a></p>\n",
        text_string(w.not_found_text),
        attr_string(&format!("{}/", cfg.url_prefix())),
        text_string(w.to_start)
    )
}

/// `site-manifest.json`: every other file with path, SHA-256, size, media type and kind; a
/// passed-through file also says whether it is active. Keys and files are sorted.
#[must_use]
pub fn manifest(
    cfg: &SiteConfig,
    revision: Option<&str>,
    ledger: &[Entry],
    dead_links: usize,
) -> String {
    let mut files: Vec<&Entry> = ledger.iter().collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let list: Vec<Value> = files
        .iter()
        .map(|e| {
            let mut v = json!({
                "path": e.path,
                "sha256": hex(&e.sha256),
                "bytes": e.bytes,
                "media": e.media,
                "kind": e.kind.word(),
            });
            if e.kind == crate::outfs::Kind::Passthrough {
                v["active"] = json!(e.active);
            }
            v
        })
        .collect();
    let langs: Vec<&str> = cfg.languages.iter().map(Lang::as_str).collect();
    let v = json!({
        "manifest": 1,
        "tool": format!("fs-text-site {}", env!("CARGO_PKG_VERSION")),
        "site": cfg.id,
        "host": cfg.host.as_str(),
        "path_prefix": cfg.url_prefix(),
        "languages": langs,
        "source_revision": revision,
        "dead_links": dead_links,
        "files": list,
    });
    let mut s = serde_json::to_string_pretty(&v).unwrap_or_default();
    s.push('\n');
    s
}
