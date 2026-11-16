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
use fs_text::import_markdown;
use fs_text_html::pass::{open_plain, PassError, PassPath};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_PAGE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_DEPTH: usize = 12;

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

fn read_small(p: &Path) -> Result<String, SiteError> {
    let m = fs::symlink_metadata(p)?;
    if !m.is_file() || m.len() > 8 * 1024 {
        return Err(SiteError::Problems(vec![format!(
            "{}: not a plain file of at most 8 KB",
            p.display()
        )]));
    }
    Ok(fs::read_to_string(p)?)
}

/// What one walk found.
#[derive(Default)]
struct Walk {
    files: Vec<String>,
    problems: Vec<String>,
}

/// All plain files below `base`, as `/`-joined relative paths, sorted. Links, devices and the
/// like are problems, never followed.
fn walk(base: &Path, rel: &str, depth: usize, w: &mut Walk) {
    let dir = if rel.is_empty() {
        base.to_path_buf()
    } else {
        base.join(rel)
    };
    let Ok(rd) = fs::read_dir(&dir) else {
        return;
    };
    let mut names: Vec<String> = Vec::new();
    for e in rd.flatten() {
        match e.file_name().into_string() {
            Ok(n) => names.push(n),
            Err(n) => w
                .problems
                .push(format!("{}: name is not UTF-8: {}", dir.display(), n.to_string_lossy())),
        }
    }
    names.sort();
    for n in names {
        let r = if rel.is_empty() {
            n.clone()
        } else {
            format!("{rel}/{n}")
        };
        let Ok(m) = fs::symlink_metadata(base.join(&r)) else {
            continue;
        };
        if m.file_type().is_symlink() {
            w.problems.push(format!("{r}: symbolic link"));
        } else if m.is_dir() {
            if depth >= MAX_DEPTH {
                w.problems.push(format!("{r}: too deep"));
            } else {
                walk(base, &r, depth + 1, w);
            }
        } else if m.is_file() {
            w.files.push(r);
        } else {
            w.problems.push(format!("{r}: not a plain file or folder"));
        }
    }
}

fn is_md(f: &str) -> bool {
    Path::new(f)
        .extension()
        .is_some_and(|e| e == "md")
}

fn url_segment_ok(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('.')
        && s.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}

/// The names of a `.pages` file's `nav:` list, in order (`- name`, `- Title: name`; `...` and
/// anything else is ignored).
fn pages_order(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for l in text.lines() {
        let Some(v) = l.trim().strip_prefix("- ") else {
            continue;
        };
        let v = v.rsplit(": ").next().unwrap_or(v).trim().trim_matches(['"', '\'']);
        let v = v.strip_suffix(".md").unwrap_or(v).trim_end_matches('/');
        if !v.is_empty() && v != "..." {
            out.push(v.to_string());
        }
    }
    out
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
        let path = match PassPath::parse(rel) {
            Ok(p) => p,
            Err(e) => {
                problems.push(format!("{origin}: {e}"));
                return None;
            }
        };
        let segs: Vec<&str> = rel.split('/').collect();
        let (dirs, file) = segs.split_at(segs.len() - 1);
        let stem = file[0].strip_suffix(".md").unwrap_or(file[0]);
        let mut url: Vec<String> = dirs.iter().map(|s| (*s).to_string()).collect();
        let is_index = stem == "index";
        if !is_index {
            url.push(stem.to_string());
        }
        if let Some(bad) = url.iter().find(|s| !url_segment_ok(s)) {
            problems.push(format!(
                "{origin}: address part {bad:?} must be lower case a-z 0-9 . _ - (nothing is renamed silently)"
            ));
            return None;
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
    f.take(MAX_PAGE_BYTES + 1).read_to_string(&mut s).map_err(|e| {
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
            for f in w.files.iter().filter(|f| f.rsplit('/').next() == Some(".pages")) {
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
                    Err(e) => out.problems.push(format!("{}/{}/{f}: {e}", self.content, lang.as_str())),
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
