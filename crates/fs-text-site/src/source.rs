//! The port of the site builder: where pages and files come from.
//!
//! Pattern: Ports and Adapters. [`SiteSource`] is the port; [`crate::FolderSource`] (a released
//! folder state, stage 1) is the adapter. Stage 2 adds an adapter on the store and on
//! fs-files (`FileRef` with a checksum); the builder does not change. A source hands out
//! *entries*, and the bytes only through [`SiteSource::open`].

use crate::config::{Lang, SiteConfig};
use fs_text::Document;
use fs_text_html::pass::{PassError, PassPath};
use std::io::Read;

/// A page in one language.
#[derive(Debug, Clone)]
pub struct PageEntry {
    /// Language of this variant.
    pub lang: Lang,
    /// Address segments below the language (empty = the start page). Lower-case `[a-z0-9._-]`.
    pub path: Vec<String>,
    /// The folder the text lies in (for relative links), as segments below the language.
    pub dir: Vec<String>,
    /// The parsed text.
    pub document: Document,
    /// Position among its siblings (`u32::MAX` = not named, natural order).
    pub order: u32,
    /// Where it came from, for messages.
    pub origin: String,
    /// Stage 2: the revision this variant was translated from; stage 1 has none.
    pub source_rev: Option<String>,
}

/// Which part of the source a file belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Area {
    /// A file a page of this language may use (image, download), path relative to the language.
    Media(Lang),
    /// A file of the user that is published at its own path, byte for byte.
    Pass,
}

/// A file of the source.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// The part.
    pub area: Area,
    /// The checked path (below the part).
    pub path: PassPath,
    /// The checksum the source declares (a stage 2 `FileRef` has one); the builder refuses a
    /// file whose bytes differ.
    pub declared_sha256: Option<[u8; 32]>,
}

/// Entries plus what could not be listed (all of it is reported, nothing is skipped silently).
#[derive(Debug)]
pub struct Listing<T> {
    /// The usable entries.
    pub items: Vec<T>,
    /// Problems, one line each.
    pub problems: Vec<String>,
}

impl<T> Default for Listing<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            problems: Vec::new(),
        }
    }
}

/// Where a site comes from.
pub trait SiteSource {
    /// The checked `site.toml`.
    fn config(&self) -> &SiteConfig;
    /// The state of the source (a commit), written into the manifest.
    fn revision(&self) -> Option<&str> {
        None
    }
    /// All pages.
    fn pages(&self) -> Listing<PageEntry>;
    /// All media files of pages.
    fn media(&self) -> Listing<FileEntry>;
    /// All files that are passed through.
    fn passthrough(&self) -> Listing<FileEntry>;
    /// The bytes of a file.
    fn open(&self, f: &FileEntry) -> Result<Box<dyn Read>, PassError>;
}
