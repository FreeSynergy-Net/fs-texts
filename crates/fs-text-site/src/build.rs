//! The site builder.
//!
//! Pattern: Builder with two phases. Phase 1 (`prepare`) reads, checks and renders everything in
//! memory and collects *all* problems; if one stops the build, nothing was written. Phase 2
//! (`write`) writes the output (new files only) and the manifest, and fixes modes and times. A
//! failed phase 2 removes what this run wrote.

use crate::error::SiteError;
use crate::index::SiteIndex;
use crate::outfs::Out;
use crate::prepare::{check_pass, register_media, render_all};
use crate::report::{BuildReport, Policy};
use crate::source::{PageEntry, SiteSource};
use crate::words::SiteWords;
use crate::write::Writer;
use fs_text_html::pass::Limits;
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
    for p in pages
        .problems
        .iter()
        .chain(&media.problems)
        .chain(&pass.problems)
    {
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
                format!(
                    "the address part {:?} is the generated contents page",
                    w.contents_slug
                ),
                true,
            );
        }
        if index
            .pages
            .insert((e.lang.clone(), e.path.clone()), i)
            .is_some()
        {
            rep.note(
                &e.origin,
                "the same page twice (a/b.md and a/b/index.md)",
                true,
            );
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
    let result = Writer::new(src, &mut out, &index, limits, &rendered).run(&passed, opt, &mut rep);
    match result {
        Ok(()) => Ok(rep),
        Err(e) => {
            out.abandon();
            Err(e)
        }
    }
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
