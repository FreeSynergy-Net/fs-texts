//! The adapter for a released folder state.
//!
//! Layout:
//! ```text
//! <root>/site.toml                    the site (schema 1 or 2)
//! <root>/content/<lang>/**/*.md       the pages (`a/b.md` and `a/b/index.md` are the page `a/b`)
//! <root>/content/<lang>/**/.pages     optional order of the siblings (`nav:` list)
//! <root>/content/<lang>/**/*.<ext>    media of the pages (images, downloads)
//! <root>/files/**                     files of the user, published at the same path, byte for byte
//! ```
//! Pattern: Adapter (of the port [`SiteSource`]). Nothing is followed through a link; names
//! that do not fit the rules are listed, not rewritten.

use crate::config::{Lang, SiteConfig};
use crate::error::SiteError;
use crate::source::{Area, FileEntry, Listing, PageEntry, SiteSource};
use crate::walk::{is_md, pages_order, read_small, suggest_segment, url_segment_ok, walk, Walk};
use fs_text::import_markdown;
use fs_text_html::pass::{open_plain, PassError, PassPath};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_PAGE_BYTES: u64 = 2 * 1024 * 1024;

/// A folder as a source.
#[derive(Debug)]
pub struct FolderSource {
    root: PathBuf,
    content: String,
    files: String,
    config: SiteConfig,
    revision: Option<String>,
}

impl FolderSource {
    /// Open the folder: reads and checks `site.toml`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, SiteError> {
        let root = root.into();
        let text = read_small(&root.join("site.toml"))?;
        let config = SiteConfig::parse(&text).map_err(|e| SiteError::Config(vec![e]))?;
        Ok(Self {
            root,
            content: "content".into(),
            files: "files".into(),
            config,
            revision: None,
        })
    }

    /// Other names for the two folders (for example `docs` for the text of fs-documentation).
    #[must_use]
    pub fn folders(mut self, content: &str, files: &str) -> Self {
        self.content = content.into();
        self.files = files.into();
        self
    }

    /// The state the folder was taken from (a commit).
    #[must_use]
    pub fn with_revision(mut self, rev: &str) -> Self {
        self.revision = Some(rev.into());
        self
    }

    fn area_root(&self, a: &Area) -> PathBuf {
        match a {
            Area::Pass => self.root.join(&self.files),
            Area::Media(l) => self.root.join(&self.content).join(l.as_str()),
        }
    }
}

impl FolderSource {
    fn page_of(
        &self,
        lang: &Lang,
        base: &Path,
        rel: &str,
        orders: &BTreeMap<String, Vec<String>>,
        problems: &mut Vec<String>,
    ) -> Option<PageEntry> {
        let origin = format!("{}/{}/{rel}", self.content, lang.as_str());
        let segs: Vec<&str> = rel.split('/').collect();
        // one message for every bad address part, with a name that would do
        let last = segs.len() - 1;
        let bad = segs.iter().enumerate().find_map(|(i, s)| {
            let part = if i == last {
                s.strip_suffix(".md").unwrap_or(s)
            } else {
                s
            };
            (!url_segment_ok(part)).then_some(part)
        });
        if let Some(bad) = bad {
            problems.push(format!(
                "{origin}: address part {bad:?} must be lower case a-z 0-9 . _ - (nothing is renamed silently; for example {:?})",
                suggest_segment(bad)
            ));
            return None;
        }
        let path = match PassPath::parse(rel) {
            Ok(p) => p,
            Err(e) => {
                problems.push(format!("{origin}: {e}"));
                return None;
            }
        };
        let (dirs, file) = segs.split_at(segs.len() - 1);
        let stem = file[0].strip_suffix(".md").unwrap_or(file[0]);
        let mut url: Vec<String> = dirs.iter().map(|s| (*s).to_string()).collect();
        let is_index = stem == "index";
        if !is_index {
            url.push(stem.to_string());
        }
        let order = if url.is_empty() {
            0
        } else {
            let (parent, name) = url.split_at(url.len() - 1);
            orders
                .get(&parent.join("/"))
                .and_then(|o| o.iter().position(|n| *n == name[0]))
                .map_or(u32::MAX, |i| u32::try_from(i).unwrap_or(u32::MAX - 1))
        };
        let text = match read_page(base, &path) {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("{origin}: {e}"));
                return None;
            }
        };
        Some(PageEntry {
            lang: lang.clone(),
            path: url,
            dir: dirs.iter().map(|s| (*s).to_string()).collect(),
            document: import_markdown(&text),
            order,
            origin,
            source_rev: None,
        })
    }
}

fn read_page(base: &Path, p: &PassPath) -> Result<String, PassError> {
    let f = open_plain(base, p)?;
    let mut s = String::new();
    f.take(MAX_PAGE_BYTES + 1)
        .read_to_string(&mut s)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidData {
                PassError::ContentMismatch("not valid UTF-8")
            } else {
                PassError::Io(e)
            }
        })?;
    if s.len() as u64 > MAX_PAGE_BYTES {
        return Err(PassError::TooBig(MAX_PAGE_BYTES));
    }
    Ok(s)
}

impl FolderSource {
    fn lang_dirs(&self, problems: &mut Vec<String>) -> Vec<Lang> {
        let base = self.root.join(&self.content);
        let mut found = Vec::new();
        let Ok(rd) = fs::read_dir(&base) else {
            return found;
        };
        let mut names: Vec<String> = rd
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        for n in names {
            match Lang::parse(&n) {
                Ok(l) if self.config.languages.contains(&l) => found.push(l),
                _ => problems.push(format!(
                    "{}/{n}: not one of the languages of site.toml",
                    self.content
                )),
            }
        }
        found
    }
}

impl SiteSource for FolderSource {
    fn config(&self) -> &SiteConfig {
        &self.config
    }

    fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    fn pages(&self) -> Listing<PageEntry> {
        let mut out = Listing::default();
        for lang in self.lang_dirs(&mut out.problems) {
            let base = self.area_root(&Area::Media(lang.clone()));
            let mut w = Walk::default();
            walk(&base, "", 0, &mut w);
            out.problems.extend(w.problems);
            let mut orders: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for f in w
                .files
                .iter()
                .filter(|f| f.rsplit('/').next() == Some(".pages"))
            {
                let dir = f.strip_suffix(".pages").unwrap_or("").trim_end_matches('/');
                match read_small(&base.join(f)) {
                    Ok(t) => {
                        orders.insert(dir.to_string(), pages_order(&t));
                    }
                    Err(e) => out.problems.push(format!("{f}: {e}")),
                }
            }
            for f in w.files.iter().filter(|f| is_md(f)) {
                if let Some(p) = self.page_of(&lang, &base, f, &orders, &mut out.problems) {
                    out.items.push(p);
                }
            }
        }
        out
    }

    fn media(&self) -> Listing<FileEntry> {
        let mut out = Listing::default();
        for lang in self.lang_dirs(&mut out.problems) {
            let base = self.area_root(&Area::Media(lang.clone()));
            let mut w = Walk::default();
            walk(&base, "", 0, &mut w);
            out.problems.extend(w.problems);
            for f in w
                .files
                .iter()
                .filter(|f| !is_md(f) && f.rsplit('/').next() != Some(".pages"))
            {
                match PassPath::parse(f) {
                    Ok(path) => out.items.push(FileEntry {
                        area: Area::Media(lang.clone()),
                        path,
                        declared_sha256: None,
                    }),
                    Err(e) => {
                        let m = format!("{}/{}/{f}: {e}", self.content, lang.as_str());
                        out.problems.push(m);
                    }
                }
            }
        }
        out
    }

    fn passthrough(&self) -> Listing<FileEntry> {
        let mut out = Listing::default();
        let base = self.area_root(&Area::Pass);
        let mut w = Walk::default();
        walk(&base, "", 0, &mut w);
        out.problems.extend(w.problems);
        for f in &w.files {
            match PassPath::parse(f) {
                Ok(path) => out.items.push(FileEntry {
                    area: Area::Pass,
                    path,
                    declared_sha256: None,
                }),
                Err(e) => out.problems.push(format!("{}/{f}: {e}", self.files)),
            }
        }
        out
    }

    fn open(&self, f: &FileEntry) -> Result<Box<dyn Read>, PassError> {
        Ok(Box::new(open_plain(&self.area_root(&f.area), &f.path)?))
    }
}
