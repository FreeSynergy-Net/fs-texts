//! All paths of one output, checked together.
//!
//! Pattern: Specification over a set — a path is refused when it repeats another (also in
//! another letter case), is a file where another is a folder, or lies under a prefix the site
//! generates. Nothing is ever overwritten.

use super::error::PassError;
use super::path::PassPath;
use std::collections::BTreeMap;

/// Paths no pass-through may ever use, whatever the site: the generated style sheet and media
/// folder, the files every site gets, and the ACME challenge folder (the web server owns it).
/// More than one segment is allowed.
pub const ALWAYS_RESERVED: [&str; 5] = [
    "_fs",
    "site.toml",
    "site-manifest.json",
    "sitemap.xml",
    ".well-known/acme-challenge",
];

/// The paths accepted so far.
#[derive(Debug, Default)]
pub struct PassSet {
    reserved: Vec<String>,
    files: BTreeMap<String, String>,
    dirs: BTreeMap<String, String>,
}

impl PassSet {
    /// A set that refuses everything under [`ALWAYS_RESERVED`] and under the given names
    /// (`de`, `en`, a nested site's folder, …; several segments allowed).
    #[must_use]
    pub fn new(reserved: &[&str]) -> Self {
        Self {
            reserved: ALWAYS_RESERVED
                .iter()
                .chain(reserved)
                .map(|s| (*s).to_string())
                .collect(),
            ..Self::default()
        }
    }

    /// Add `p`, or say why not.
    pub fn add(&mut self, p: &PassPath) -> Result<(), PassError> {
        if let Some(r) = self.reserved.iter().find(|r| p.is_under(r)) {
            return Err(PassError::Reserved(r.clone()));
        }
        let key = p.as_str().to_ascii_lowercase();
        if self.files.contains_key(&key) {
            return Err(PassError::Duplicate(p.as_str().to_string()));
        }
        if self.dirs.contains_key(&key) {
            return Err(PassError::FileAndFolder(p.as_str().to_string()));
        }
        let mut dirs = Vec::new();
        let mut acc = String::new();
        let n = key.matches('/').count();
        for (i, seg) in key.split('/').enumerate() {
            if i == n {
                break;
            }
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(seg);
            if self.files.contains_key(&acc) {
                return Err(PassError::FileAndFolder(p.as_str().to_string()));
            }
            dirs.push(acc.clone());
        }
        for d in dirs {
            self.dirs.entry(d).or_insert_with(|| p.as_str().to_string());
        }
        self.files.insert(key, p.as_str().to_string());
        Ok(())
    }
}
