//! The output folder: files written new, listed for the manifest, and made reproducible.
//!
//! Pattern: Unit of work with a ledger. Every file that ends up in the output is recorded
//! (path, SHA-256, size, type, kind); at the end the manifest is written from the ledger and
//! the whole tree gets fixed modes and one modification time, so two builds of the same source
//! give the same bytes, modes and times.

use crate::error::SiteError;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What a file of the output is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A page the builder rendered from text.
    Page,
    /// A media file of a page, named by its hash.
    Media,
    /// A file of the user, byte for byte.
    Passthrough,
    /// Anything else the builder wrote (style sheet, sitemap, 404, site.toml, …).
    Generated,
}

impl Kind {
    /// The word in the manifest.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Kind::Page => "page",
            Kind::Media => "media",
            Kind::Passthrough => "passthrough",
            Kind::Generated => "generated",
        }
    }
}

/// One line of the ledger.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Path below the output folder.
    pub path: String,
    /// SHA-256.
    pub sha256: [u8; 32],
    /// Size.
    pub bytes: u64,
    /// Media type sent for it.
    pub media: String,
    /// The kind.
    pub kind: Kind,
    /// Can the browser run something from it (passthrough only)?
    pub active: bool,
}

/// The output folder being written.
pub struct Out {
    root: PathBuf,
    created_root: bool,
    /// What was written.
    pub ledger: Vec<Entry>,
}

/// Lower-case hex.
#[must_use]
pub fn hex(d: &[u8]) -> String {
    use std::fmt::Write;
    d.iter().fold(String::new(), |mut o, b| {
        let _ = write!(o, "{b:02x}");
        o
    })
}

impl Out {
    /// Take `root`: it must not exist, or be an empty folder. Nothing existing is ever
    /// overwritten.
    pub fn create(root: &Path) -> Result<Self, SiteError> {
        let created_root = match fs::symlink_metadata(root) {
            Ok(m) if m.is_dir() => {
                if fs::read_dir(root)?.next().is_some() {
                    return Err(SiteError::OutputNotEmpty(root.display().to_string()));
                }
                false
            }
            Ok(_) => return Err(SiteError::OutputNotEmpty(root.display().to_string())),
            Err(_) => {
                fs::create_dir_all(root)?;
                true
            }
        };
        Ok(Self {
            root: root.to_path_buf(),
            created_root,
            ledger: Vec::new(),
        })
    }

    /// The folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Write `bytes` at `path` (new file) and record it.
    pub fn put(
        &mut self,
        path: &str,
        bytes: &[u8],
        media: &str,
        kind: Kind,
    ) -> Result<(), SiteError> {
        let to = self.root.join(path);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = OpenOptions::new().write(true).create_new(true).open(&to)?;
        f.write_all(bytes)?;
        self.ledger.push(Entry {
            path: path.to_string(),
            sha256: Sha256::digest(bytes).into(),
            bytes: bytes.len() as u64,
            media: media.to_string(),
            kind,
            active: false,
        });
        Ok(())
    }

    /// Record a file another part wrote into the folder (the pass-through copy).
    pub fn record(&mut self, e: Entry) {
        self.ledger.push(e);
    }

    /// Fixed modes and one modification time for every file and folder.
    pub fn finish(&self, mtime: SystemTime) -> Result<(), SiteError> {
        fn walk(dir: &Path, mtime: SystemTime, dirs: &mut Vec<PathBuf>) -> std::io::Result<()> {
            for e in fs::read_dir(dir)? {
                let p = e?.path();
                if fs::symlink_metadata(&p)?.is_dir() {
                    walk(&p, mtime, dirs)?;
                    dirs.push(p);
                } else {
                    set_mode(&p, false)?;
                    File::open(&p)?.set_modified(mtime)?;
                }
            }
            Ok(())
        }
        let mut dirs = Vec::new();
        walk(&self.root, mtime, &mut dirs)?;
        dirs.push(self.root.clone());
        for d in dirs {
            set_mode(&d, true)?;
            File::open(&d)?.set_modified(mtime)?;
        }
        Ok(())
    }

    /// Remove what this run made (after a failed build); only if it made the folder.
    pub fn abandon(self) {
        if self.created_root {
            let _ = fs::remove_dir_all(&self.root);
        } else if let Ok(rd) = fs::read_dir(&self.root) {
            for e in rd.flatten() {
                let p = e.path();
                let _ = if p.is_dir() {
                    fs::remove_dir_all(&p)
                } else {
                    fs::remove_file(&p)
                };
            }
        }
    }
}

#[cfg(unix)]
fn set_mode(p: &Path, dir: bool) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(p, fs::Permissions::from_mode(if dir { 0o755 } else { 0o644 }))
}

#[cfg(not(unix))]
fn set_mode(_: &Path, _: bool) -> std::io::Result<()> {
    Ok(())
}
