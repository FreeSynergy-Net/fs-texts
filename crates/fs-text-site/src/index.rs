//! The table of everything a page may point to, and the pretty-link resolvers built on it.
//!
//! Pattern: Strategy implementations ([`PageLinks`], [`PageMedia`] of `fs-text-html`'s
//! `LinkResolver` and `MediaResolver`) over one shared, read-only index (`Arc`). Resolution
//! is by whole path segments; nothing is guessed (no letter-case fix, no `.html`).

use crate::config::Lang;
use crate::source::FileEntry;
use fs_text::{LinkStyle, LinkValue};
use fs_text_html::pass::MediaType;
use fs_text_html::{LinkResolver, LinkTarget, MediaResolver, MediaTarget};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A media file of the source, hashed, with the address it gets.
#[derive(Debug, Clone)]
pub struct MediaRef {
    /// File name in `/_fs/media/` (`<sha256 16 hex>.<ext>`).
    pub name: String,
    /// Size.
    pub bytes: u64,
    /// SHA-256.
    pub sha256: [u8; 32],
    /// Media type.
    pub media: MediaType,
    /// Where the bytes come from.
    pub entry: FileEntry,
}

/// What pages and media exist, per language.
#[derive(Debug, Default)]
pub struct SiteIndex {
    /// `""` or `/a/b`.
    pub prefix: String,
    /// The language of the fallback.
    pub default_lang: Option<Lang>,
    /// `(lang, address segments)` of every page.
    pub pages: BTreeMap<(Lang, Vec<String>), usize>,
    /// First segments below the site at which other sites of the same host lie: an absolute
    /// link into one of them is a link to that site, not a dead one.
    pub other_sites: Vec<String>,
    /// `(lang, path below the language)` of every media file.
    pub media: BTreeMap<(Lang, String), MediaRef>,
}

/// The address of a page: `{prefix}/{lang}/a/b/`.
#[must_use]
pub fn page_url(prefix: &str, lang: &Lang, path: &[String]) -> String {
    let mut u = format!("{prefix}/{}/", lang.as_str());
    for s in path {
        u.push_str(s);
        u.push('/');
    }
    u
}

/// Address of a media file in the output.
#[must_use]
pub fn media_url(prefix: &str, name: &str) -> String {
    format!("{prefix}/_fs/media/{name}")
}

/// Join `target` (as written) to a list of segments below the language root. `base` is the
/// folder of the text for a relative target; a target that starts with `/` and a wiki link
/// start at the root. `None` if it climbs out of the root or has an odd segment.
fn join(base: &[String], target: &str, from_root: bool) -> Option<Vec<String>> {
    let (t, absolute) = match target.strip_prefix('/') {
        Some(r) => (r, true),
        None => (target, false),
    };
    let mut segs: Vec<String> = if absolute || from_root {
        Vec::new()
    } else {
        base.to_vec()
    };
    for s in t.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                segs.pop()?;
            }
            s if s.contains(['\\', '%', '?', ':']) || s.chars().any(char::is_control) => {
                return None
            }
            s => segs.push(s.to_string()),
        }
    }
    Some(segs)
}

/// `a/b.md`, `a/b/index.md`, `a/b/`, `a/b` are all the page `a/b`.
fn page_segments(mut segs: Vec<String>) -> Vec<String> {
    if let Some(last) = segs.last_mut() {
        if let Some(stem) = last.strip_suffix(".md") {
            *last = stem.to_string();
        }
    }
    if segs.last().is_some_and(|l| l == "index") {
        segs.pop();
    }
    segs
}

impl SiteIndex {
    fn langs<'a>(&'a self, lang: &'a Lang) -> impl Iterator<Item = &'a Lang> {
        let d = self.default_lang.as_ref().filter(|d| *d != lang);
        std::iter::once(lang).chain(d)
    }

    fn find_page(&self, lang: &Lang, base: &[String], link: &LinkValue) -> Option<String> {
        let (path, frag) = link.split_fragment();
        let segs = page_segments(join(base, path, link.style == LinkStyle::Wiki)?);
        for l in self.langs(lang) {
            if self.pages.contains_key(&(l.clone(), segs.clone())) {
                let mut u = page_url(&self.prefix, l, &segs);
                if let Some(f) = frag.filter(|f| !f.is_empty()) {
                    u.push('#');
                    u.push_str(f);
                }
                return Some(u);
            }
        }
        None
    }

    fn find_media(&self, lang: &Lang, base: &[String], link: &LinkValue) -> Option<&MediaRef> {
        let (path, _) = link.split_fragment();
        let segs = join(base, path, link.style == LinkStyle::Embed)?;
        let key = segs.join("/");
        self.langs(lang)
            .find_map(|l| self.media.get(&(l.clone(), key.clone())))
    }
}

/// Where the links of one page point.
#[derive(Clone)]
pub struct PageLinks {
    /// The shared index.
    pub index: Arc<SiteIndex>,
    /// Language of the page.
    pub lang: Lang,
    /// Folder of the page's text.
    pub dir: Vec<String>,
}

impl LinkResolver for PageLinks {
    fn resolve(&self, link: &LinkValue) -> LinkTarget {
        let p = link.path.trim();
        if p.is_empty() {
            return LinkTarget::Missing;
        }
        if p.starts_with('#') {
            return LinkTarget::Href(p.to_string());
        }
        if has_scheme(p) {
            return LinkTarget::Href(p.to_string());
        }
        if let Some(rest) = p.strip_prefix('/') {
            let first = rest.split('/').next().unwrap_or_default();
            if self.index.other_sites.iter().any(|s| s == first) {
                return LinkTarget::Href(format!("{}{p}", self.index.prefix));
            }
        }
        if let Some(u) = self.index.find_page(&self.lang, &self.dir, link) {
            return LinkTarget::Href(u);
        }
        match self.index.find_media(&self.lang, &self.dir, link) {
            Some(m) => LinkTarget::Href(media_url(&self.index.prefix, &m.name)),
            None => LinkTarget::Missing,
        }
    }
}

fn has_scheme(p: &str) -> bool {
    p.find([':', '/', '?', '#'])
        .is_some_and(|i| p.as_bytes()[i] == b':')
}

/// Where the images of one page are.
#[derive(Clone)]
pub struct PageMedia {
    /// The shared index.
    pub index: Arc<SiteIndex>,
    /// Language of the page.
    pub lang: Lang,
    /// Folder of the page's text.
    pub dir: Vec<String>,
}

impl MediaResolver for PageMedia {
    fn resolve(&self, media: &LinkValue) -> Option<MediaTarget> {
        let m = self.index.find_media(&self.lang, &self.dir, media)?;
        Some(MediaTarget {
            src: media_url(&self.index.prefix, &m.name),
            width: None,
            height: None,
        })
    }
}
