//! The site builder.
//!
//! Pattern: Builder with two phases. Phase 1 reads, checks and renders everything in memory and
//! collects *all* problems; if one stops the build, nothing was written. Phase 2 writes the
//! output (new files only) and the manifest, and fixes modes and times. A failed phase 2 removes
//! what this run wrote.

use crate::config::{CspMode, Lang, SiteConfig};
use crate::error::SiteError;
use crate::gen;
use crate::index::{media_url, page_url, MediaRef, PageLinks, PageMedia, SiteIndex};
use crate::outfs::{hex, Entry, Kind, Out};
use crate::report::{BuildReport, Gate, Policy};
use crate::sites::GENERATED_TOP;
use crate::source::{Area, FileEntry, PageEntry, SiteSource};
use crate::tree::{build as build_tree, render_full, TreePage};
use crate::words::SiteWords;
use fs_text_html::pass::{copy_from, measure, Limits, MediaType, PassPath, PassSet};
use fs_text_html::{
    render_page, theme, Alternate, Csp, Frame, HtmlExporterBuilder, HtmlPage, NavNode, Navigation,
    Problem,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

/// Settings of one build.
#[derive(Clone, Debug)]
pub struct BuildOptions {
    /// Modification time of every file and folder of the output (the release time of the
    /// source, `SITE_MTIME`); never the clock of the machine, so builds repeat.
    pub mtime: SystemTime,
    /// What stops the build.
    pub policy: Policy,
    /// Biggest file (pass-through and media).
    pub max_file_bytes: u64,
    /// Names below this site at which other sites of the same host lie (from the `SiteSet`);
    /// this site's pass-through may not use them.
    pub nested: Vec<String>,
}

impl BuildOptions {
    /// Default settings with the given modification time.
    #[must_use]
    pub fn new(mtime: SystemTime) -> Self {
        Self {
            mtime,
            policy: Policy::default(),
            max_file_bytes: Limits::default().max_bytes,
            nested: Vec::new(),
        }
    }
}

/// Media a page may use: pictures and documents that run nothing.
fn page_media_ok(m: MediaType) -> bool {
    matches!(
        m,
        MediaType::Png
            | MediaType::Jpeg
            | MediaType::Gif
            | MediaType::Webp
            | MediaType::Avif
            | MediaType::Pdf
            | MediaType::Txt
    )
}

struct Rendered {
    entry: PageEntry,
    html: HtmlPage,
}

/// Build the site `src` into the new folder `out_dir`.
pub fn build(
    src: &dyn SiteSource,
    out_dir: &Path,
    opt: &BuildOptions,
) -> Result<BuildReport, SiteError> {
    let cfg = src.config();
    let mut rep = BuildReport::default();
    let limits = Limits {
        max_bytes: opt.max_file_bytes,
    };
    let prefix = cfg.url_prefix();

    // ---- phase 1: read, check, render (nothing is written)
    let pages = src.pages();
    let media = src.media();
    let pass = src.passthrough();
    for p in pages.problems.iter().chain(&media.problems).chain(&pass.problems) {
        rep.note("source", p.clone(), true);
    }

    let passed = check_pass(src, cfg, &pass.items, opt, limits, &mut rep);
    let mut index = SiteIndex {
        prefix: prefix.clone(),
        default_lang: Some(cfg.default_language.clone()),
        ..SiteIndex::default()
    };
    register_media(src, &media.items, limits, &mut index, &mut rep);

    let mut entries: Vec<PageEntry> = pages.items;
    entries.sort_by(|a, b| (&a.lang, &a.path).cmp(&(&b.lang, &b.path)));
    for (i, e) in entries.iter().enumerate() {
        let w = SiteWords::for_lang(e.lang.as_str());
        if e.path.first().is_some_and(|s| s == w.contents_slug) {
            rep.note(
                &e.origin,
                format!("the address part {:?} is the generated contents page", w.contents_slug),
                true,
            );
        }
        if index.pages.insert((e.lang.clone(), e.path.clone()), i).is_some() {
            rep.note(&e.origin, "the same page twice (a/b.md and a/b/index.md)", true);
        }
    }
    let index = Arc::new(index);
    let rendered = render_all(entries, &index, opt, &mut rep);
    if rep.has_fatal() {
        return Err(SiteError::Problems(
            rep.findings
                .iter()
                .filter(|f| f.fatal)
                .map(|f| format!("{}: {}", f.place, f.message))
                .collect(),
        ));
    }

    // ---- phase 2: write
    let mut out = Out::create(out_dir)?;
    match write_all(src, &mut out, &index, &passed, &rendered, opt, limits, &mut rep) {
        Ok(()) => Ok(rep),
        Err(e) => {
            out.abandon();
            Err(e)
        }
    }
}

/// Checks of the pass-through files (path set, `.well-known`, active files against the CSP
/// mode, content and checksum). Returns what passed, with its measurement.
fn check_pass(
    src: &dyn SiteSource,
    cfg: &SiteConfig,
    items: &[FileEntry],
    opt: &BuildOptions,
    limits: Limits,
    rep: &mut BuildReport,
) -> Vec<(FileEntry, [u8; 32], bool)> {
    let supplied: BTreeSet<String> = items.iter().map(|f| f.path.as_str().to_string()).collect();
    let mut reserved: Vec<String> = GENERATED_TOP
        .iter()
        .filter(|n| !matches!(**n, "index.html" | "404.html" | "robots.txt"))
        .map(|n| (*n).to_string())
        .collect();
    for n in ["index.html", "404.html"] {
        if !supplied.contains(n) {
            reserved.push(n.to_string());
        }
    }
    if cfg.path_prefix.is_root() && !supplied.contains("robots.txt") {
        reserved.push("robots.txt".into());
    }
    reserved.extend(cfg.languages.iter().map(|l| l.as_str().to_string()));
    reserved.extend(opt.nested.iter().cloned());
    let names: Vec<&str> = reserved.iter().map(String::as_str).collect();
    let mut set = PassSet::new(&names);
    let mut ok = Vec::new();
    for f in items {
        let place = format!("files/{}", f.path.as_str());
        if let Err(e) = set.add(&f.path) {
            rep.note(&place, e.to_string(), true);
            continue;
        }
        if f.path.is_under(".well-known") && f.path.as_str() != ".well-known/security.txt" {
            rep.note(&place, "only .well-known/security.txt may be passed through", true);
            continue;
        }
        let mut r = match src.open(f) {
            Ok(r) => r,
            Err(e) => {
                rep.note(&place, e.to_string(), true);
                continue;
            }
        };
        match measure(&mut r, &f.path, limits) {
            Ok(c) => {
                if f.declared_sha256.is_some_and(|d| d != c.sha256) {
                    rep.note(&place, "the bytes do not match the checksum the source declares", true);
                    continue;
                }
                if c.media.is_active() && cfg.csp_mode == CspMode::Static {
                    rep.note(
                        &place,
                        "an active file (HTML, script or SVG) needs csp.mode = \"passthrough\" in site.toml",
                        true,
                    );
                    continue;
                }
                ok.push((f.clone(), c.sha256, c.media.is_active()));
            }
            Err(e) => rep.note(&place, e.to_string(), true),
        }
    }
    ok
}

fn register_media(
    src: &dyn SiteSource,
    items: &[FileEntry],
    limits: Limits,
    index: &mut SiteIndex,
    rep: &mut BuildReport,
) {
    for f in items {
        let Area::Media(lang) = &f.area else { continue };
        let place = format!("content/{}/{}", lang.as_str(), f.path.as_str());
        let mut r = match src.open(f) {
            Ok(r) => r,
            Err(e) => {
                rep.note(&place, e.to_string(), true);
                continue;
            }
        };
        match measure(&mut r, &f.path, limits) {
            Ok(c) => {
                if !page_media_ok(c.media) {
                    rep.note(
                        &place,
                        "this type may not be page media (pictures, PDF, text only); a file that runs something belongs under files/",
                        true,
                    );
                    continue;
                }
                if f.declared_sha256.is_some_and(|d| d != c.sha256) {
                    rep.note(&place, "the bytes do not match the checksum the source declares", true);
                    continue;
                }
                let ext = f.path.extension().unwrap_or_default();
                let name = format!("{}.{ext}", &hex(&c.sha256)[..16]);
                index.media.insert(
                    (lang.clone(), f.path.as_str().to_string()),
                    MediaRef {
                        name,
                        bytes: c.bytes,
                        sha256: c.sha256,
                        media: c.media,
                        entry: f.clone(),
                    },
                );
            }
            Err(e) => rep.note(&place, e.to_string(), true),
        }
    }
}

fn render_all(
    entries: Vec<PageEntry>,
    index: &Arc<SiteIndex>,
    opt: &BuildOptions,
    rep: &mut BuildReport,
) -> Vec<Rendered> {
    let mut out = Vec::new();
    for e in entries {
        let exporter = HtmlExporterBuilder::default()
            .links(PageLinks {
                index: index.clone(),
                lang: e.lang.clone(),
                dir: e.dir.clone(),
            })
            .media(PageMedia {
                index: index.clone(),
                lang: e.lang.clone(),
                dir: e.dir.clone(),
            })
            .lang(e.lang.as_str())
            .build();
        let html = exporter.render(&e.document);
        for p in &html.report.problems {
            note_problem(rep, &e.origin, p, opt.policy);
        }
        out.push(Rendered { entry: e, html });
    }
    out
}

fn note_problem(rep: &mut BuildReport, place: &str, p: &Problem, pol: Policy) {
    let fatal = |g: Gate| g == Gate::Fail;
    match p {
        Problem::DeadLink { path } => rep.note(
            place,
            format!("link {path:?} points to nothing (text kept, no link)"),
            fatal(pol.dead_links),
        ),
        Problem::MissingAlt { src } => rep.note(
            place,
            format!("image {src:?} has no alt text"),
            fatal(pol.missing_alt),
        ),
        Problem::UnsafeMedia { src } if !src.contains(':') => rep.note(
            place,
            format!("image file {src:?} is missing"),
            fatal(pol.missing_media),
        ),
        other => rep.note(place, format!("{other:?}"), false),
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn write_all(
    src: &dyn SiteSource,
    out: &mut Out,
    index: &Arc<SiteIndex>,
    passed: &[(FileEntry, [u8; 32], bool)],
    rendered: &[Rendered],
    opt: &BuildOptions,
    limits: Limits,
    rep: &mut BuildReport,
) -> Result<(), SiteError> {
    let cfg = src.config();
    let prefix = cfg.url_prefix();
    let origin = cfg.origin();

    // user files, byte for byte
    let mut supplied: BTreeSet<String> = BTreeSet::new();
    for (f, sha, active) in passed {
        let mut r = src.open(f).map_err(|e| SiteError::Problems(vec![e.to_string()]))?;
        let c = copy_from(&mut r, &f.path, out.root(), limits)
            .map_err(|e| SiteError::Problems(vec![format!("{}: {e}", f.path.as_str())]))?;
        if c.sha256 != *sha {
            return Err(SiteError::Problems(vec![format!(
                "{}: changed while the build ran",
                f.path.as_str()
            )]));
        }
        supplied.insert(f.path.as_str().to_string());
        out.record(Entry {
            path: f.path.as_str().to_string(),
            sha256: c.sha256,
            bytes: c.bytes,
            media: c.media.mime().to_string(),
            kind: Kind::Passthrough,
            active: *active,
        });
    }

    // style sheet
    let css_name = format!(
        "_fs/theme.{}.css",
        &hex(&Sha256::digest(theme::CSS.as_bytes()))[..16]
    );
    out.put(&css_name, theme::CSS.as_bytes(), "text/css", Kind::Generated)?;
    let css_url = format!("{prefix}/{css_name}");

    // media that pages use
    let mut used: BTreeSet<String> = BTreeSet::new();
    let media_prefix = media_url(&prefix, "");
    for r in rendered {
        for a in r.html.links.iter().chain(&r.html.media) {
            if let Some(n) = a.strip_prefix(&media_prefix) {
                used.insert(n.split('#').next().unwrap_or(n).to_string());
            }
        }
    }
    let mut written: BTreeSet<String> = BTreeSet::new();
    for m in index.media.values() {
        if used.contains(&m.name) && written.insert(m.name.clone()) {
            let rel = PassPath::parse(&format!("_fs/media/{}", m.name))
                .map_err(|e| SiteError::Problems(vec![e.to_string()]))?;
            let mut rd = src
                .open(&m.entry)
                .map_err(|e| SiteError::Problems(vec![e.to_string()]))?;
            let c = copy_from(&mut rd, &rel, out.root(), limits)
                .map_err(|e| SiteError::Problems(vec![format!("{}: {e}", rel.as_str())]))?;
            if c.sha256 != m.sha256 {
                return Err(SiteError::Problems(vec![format!(
                    "{}: changed while the build ran",
                    rel.as_str()
                )]));
            }
            out.record(Entry {
                path: rel.as_str().to_string(),
                sha256: c.sha256,
                bytes: c.bytes,
                media: c.media.mime().to_string(),
                kind: Kind::Media,
                active: false,
            });
        }
    }
    for m in index.media.values() {
        if !used.contains(&m.name) {
            rep.note(
                m.entry.path.as_str(),
                "media file no page uses (not published)",
                false,
            );
        }
    }

    // pages, per language
    let csp = cfg
        .csp_policy
        .as_ref()
        .map_or(Csp::Strict, |p| Csp::Custom(p.clone()));
    let mut by_path: BTreeMap<Vec<String>, Vec<Lang>> = BTreeMap::new();
    for r in rendered {
        by_path
            .entry(r.entry.path.clone())
            .or_default()
            .push(r.entry.lang.clone());
    }
    let mut starts: Vec<(Lang, String)> = Vec::new();
    for lang in &cfg.languages {
        let mine: Vec<&Rendered> = rendered.iter().filter(|r| &r.entry.lang == lang).collect();
        if mine.is_empty() {
            rep.note("site.toml", format!("language {} has no page", lang.as_str()), false);
            continue;
        }
        let w = SiteWords::for_lang(lang.as_str());
        let title_of = |r: &Rendered| -> String {
            r.html
                .title
                .clone()
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| {
                    r.entry
                        .path
                        .last()
                        .cloned()
                        .unwrap_or_else(|| cfg.name.clone())
                })
        };
        let tree_pages: Vec<TreePage> = mine
            .iter()
            .map(|r| TreePage {
                path: r.entry.path.clone(),
                title: title_of(r),
                order: r.entry.order,
            })
            .collect();
        let contents_path = vec![w.contents_slug.to_string()];
        let contents_url = page_url(&prefix, lang, &contents_path);
        let contents_node = NavNode {
            title: w.contents.to_string(),
            href: contents_url.clone(),
            children: Vec::new(),
        };
        let core = build_tree(&prefix, lang, &tree_pages, Vec::new());
        let nav = build_tree(&prefix, lang, &tree_pages, vec![contents_node]);
        let mut frame = Frame {
            lang: lang.as_str().to_string(),
            site_name: cfg.name.clone(),
            canonical: None,
            alternates: Vec::new(),
            x_default: Some(format!("{origin}/")),
            stylesheet: css_url.clone(),
            csp: csp.clone(),
            nav,
            current: String::new(),
            footer: None,
        };
        for r in &mine {
            let mut page = r.html.clone();
            page.title = Some(title_of(r));
            let abs = |l: &Lang| format!("https://{}{}", cfg.host.as_str(), page_url(&prefix, l, &r.entry.path));
            frame.current = page_url(&prefix, lang, &r.entry.path);
            frame.canonical = Some(abs(lang));
            frame.alternates = by_path[&r.entry.path]
                .iter()
                .map(|l| Alternate {
                    lang: l.as_str().to_string(),
                    href: abs(l),
                })
                .collect();
            let html = render_page(&frame, &page);
            put_page(out, lang, &r.entry.path, &html)?;
        }
        // contents page
        let body = render_full(&Navigation {
            roots: core.roots.clone(),
        });
        let page = HtmlPage {
            body,
            title: Some(w.contents.to_string()),
            ..HtmlPage::default()
        };
        frame.current.clone_from(&contents_url);
        let abs_c = |l: &Lang| {
            let wl = SiteWords::for_lang(l.as_str());
            format!(
                "https://{}{}",
                cfg.host.as_str(),
                page_url(&prefix, l, &[wl.contents_slug.to_string()])
            )
        };
        frame.canonical = Some(abs_c(lang));
        frame.alternates = cfg
            .languages
            .iter()
            .filter(|l| rendered.iter().any(|r| &r.entry.lang == *l))
            .map(|l| Alternate {
                lang: l.as_str().to_string(),
                href: abs_c(l),
            })
            .collect();
        put_page(out, lang, &contents_path, &render_page(&frame, &page))?;
        rep.pages += mine.len() + 1;
        let start = if mine.iter().any(|r| r.entry.path.is_empty()) {
            page_url(&prefix, lang, &[])
        } else {
            rep.note(
                &format!("content/{}", lang.as_str()),
                "no start page (index.md)",
                false,
            );
            contents_url
        };
        starts.push((lang.clone(), start));
    }

    // generated files of the site
    let plain = |lang: &Lang, current: &str, canonical: String, alternates: Vec<Alternate>| Frame {
        lang: lang.as_str().to_string(),
        site_name: cfg.name.clone(),
        canonical: Some(canonical),
        alternates,
        x_default: Some(format!("{origin}/")),
        stylesheet: css_url.clone(),
        csp: csp.clone(),
        nav: Navigation::default(),
        current: current.to_string(),
        footer: None,
    };
    let dl = &cfg.default_language;
    if !supplied.contains("index.html") {
        let page = HtmlPage {
            body: gen::choose_body(cfg, &starts),
            title: Some(cfg.name.clone()),
            ..HtmlPage::default()
        };
        let f = plain(dl, "", format!("{origin}/"), Vec::new());
        out.put("index.html", render_page(&f, &page).as_bytes(), "text/html", Kind::Generated)?;
    }
    if !supplied.contains("404.html") {
        let w = SiteWords::for_lang(dl.as_str());
        let page = HtmlPage {
            body: gen::not_found_body(cfg),
            title: Some(w.not_found.to_string()),
            ..HtmlPage::default()
        };
        let f = plain(dl, "", format!("{origin}/"), Vec::new());
        out.put("404.html", render_page(&f, &page).as_bytes(), "text/html", Kind::Generated)?;
    }
    out.put(
        "sitemap.xml",
        gen::sitemap(cfg, &by_path).as_bytes(),
        "application/xml",
        Kind::Generated,
    )?;
    if cfg.path_prefix.is_root() && !supplied.contains("robots.txt") {
        out.put("robots.txt", gen::robots(cfg).as_bytes(), "text/plain", Kind::Generated)?;
    }
    out.put("site.toml", cfg.to_schema1().as_bytes(), "application/toml", Kind::Generated)?;

    // the manifest names all the others
    let m = gen::manifest(cfg, src.revision(), &out.ledger);
    rep.files = out.ledger.len();
    rep.manifest_sha256 = hex(&Sha256::digest(m.as_bytes()));
    out.put("site-manifest.json", m.as_bytes(), "application/json", Kind::Generated)?;
    out.finish(opt.mtime)
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

/// Build every site folder below `sites_root` (each holds a `site.toml`) into
/// `out_root/<site.id>`. All `site.toml` files are checked together first: a host name or path
/// two sites claim, or an inner site that sits on a generated name, stops everything before
/// anything is written.
pub fn build_all(
    sites_root: &Path,
    out_root: &Path,
    opt: &BuildOptions,
) -> Result<Vec<(String, BuildReport)>, SiteError> {
    let mut dirs: Vec<std::path::PathBuf> = std::fs::read_dir(sites_root)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("site.toml").is_file())
        .collect();
    dirs.sort();
    let mut sources = Vec::new();
    for d in dirs {
        sources.push(crate::folder::FolderSource::open(d)?);
    }
    let set = crate::sites::SiteSet::new(sources.iter().map(|s| s.config().clone()).collect())
        .map_err(SiteError::Config)?;
    let mut out = Vec::new();
    for s in &sources {
        let mut o = opt.clone();
        o.nested = set.nested_below(s.config());
        let id = s.config().id.clone();
        let r = build(s, &out_root.join(&id), &o)?;
        out.push((id, r));
    }
    Ok(out)
}
