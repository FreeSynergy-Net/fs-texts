//! Phase 1 of the build: read, check and render, in memory.
//!
//! Pattern: a sequence of Specifications over the source (pass-through set, media table, page
//! rendering); each writes its findings into the report and nothing to disk.

use crate::build::BuildOptions;
use crate::config::CspMode;
use crate::config::SiteConfig;
use crate::index::{MediaRef, PageLinks, PageMedia, SiteIndex};
use crate::outfs::hex;
use crate::report::{BuildReport, Gate, Policy};
use crate::sites::GENERATED_TOP;
use crate::source::{Area, FileEntry, PageEntry, SiteSource};
use fs_text_html::pass::{measure, Limits, MediaType, PassSet};
use fs_text_html::{HtmlExporterBuilder, HtmlPage, Problem};
use std::collections::BTreeSet;
use std::sync::Arc;

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

pub(crate) struct Rendered {
    pub(crate) entry: PageEntry,
    pub(crate) html: HtmlPage,
}

/// Checks of the pass-through files (path set, `.well-known`, active files against the CSP
/// mode, content and checksum). Returns what passed, with its measurement.
pub(crate) fn check_pass(
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
            rep.note(
                &place,
                "only .well-known/security.txt may be passed through",
                true,
            );
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
                    rep.note(
                        &place,
                        "the bytes do not match the checksum the source declares",
                        true,
                    );
                    continue;
                }
                if c.media.is_active() && cfg.csp_mode_for(f.path.as_str()) == CspMode::Static {
                    rep.note(
                        &place,
                        "an active file (HTML, script or SVG) needs csp.mode = \"passthrough\" (or a [[csp.area]] with it for its folder) in site.toml",
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

pub(crate) fn register_media(
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
                    rep.note(
                        &place,
                        "the bytes do not match the checksum the source declares",
                        true,
                    );
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

pub(crate) fn render_all(
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
        Problem::DeadLink { path } => rep.dead_link(
            place,
            format!("link {path:?} points to nothing (text kept, no link)"),
            pol.dead_links,
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

/// A folder policy that allows more than the site's (`allow_loosen = true`) is a warning.
pub(crate) fn loosened_areas(cfg: &SiteConfig, rep: &mut BuildReport) {
    let site = cfg
        .csp_policy
        .as_deref()
        .or_else(|| fs_text_html::Csp::Strict.content())
        .unwrap_or_default();
    for a in &cfg.csp_areas {
        if a.policy
            .as_deref()
            .is_some_and(|p| crate::csp_rel::looser(site, p))
        {
            rep.note(
                "site.toml",
                format!(
                    "[[csp.area]] {}: its policy allows more than the policy of the site (allow_loosen = true)",
                    a.path.as_url()
                ),
                false,
            );
        }
    }
}
