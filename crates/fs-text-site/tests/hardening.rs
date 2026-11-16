//! Hardening probes: the longest host name, and a file that changes between the check and the copy.
mod common;
use common::*;
use fs_text_html::pass::PassError;
use fs_text_site::{
    build, FileEntry, FolderSource, Host, Listing, PageEntry, SiteConfig, SiteSource,
};
use std::cell::Cell;
use std::io::{Cursor, Read};

// ---- host length ------------------------------------------------------------------------

fn host_of_len(n: usize) -> String {
    // labels of 63 characters, the last one the top-level label
    let mut labels = Vec::new();
    let mut left = n;
    while left > 0 {
        let l = left.min(63);
        labels.push("a".repeat(l));
        left = left.saturating_sub(l + 1);
    }
    let h = labels.join(".");
    assert_eq!(h.len(), n, "{h}");
    h
}

#[test]
fn a_host_of_253_bytes_is_accepted_and_254_is_refused() {
    let ok = host_of_len(253);
    assert!(Host::parse(&ok).is_ok(), "253");
    let long = host_of_len(254);
    let e = Host::parse(&long).unwrap_err().to_string();
    assert!(e.contains("253"), "{e}");
    // through a whole site.toml as well
    let toml = |h: &str| {
        format!("schema = 2\npurpose = \"website\"\n[site]\nid = \"s\"\n[names]\nprimary = \"{h}\"\n[acme]\nmethod = \"http-01\"\n")
    };
    // (the file limit of 8 KB is far above this)
    assert!(SiteConfig::parse(&toml(&ok)).is_ok());
    assert!(SiteConfig::parse(&toml(&long)).is_err());
}

// ---- a file that changes between the check and the copy -----------------------------------

/// A source whose passed file shows other bytes the second time it is opened.
struct Flaky {
    inner: FolderSource,
    opens: Cell<usize>,
    flip: bool,
}

impl SiteSource for Flaky {
    fn config(&self) -> &SiteConfig {
        self.inner.config()
    }
    fn pages(&self) -> Listing<PageEntry> {
        self.inner.pages()
    }
    fn media(&self) -> Listing<FileEntry> {
        self.inner.media()
    }
    fn passthrough(&self) -> Listing<FileEntry> {
        self.inner.passthrough()
    }
    fn open(&self, f: &FileEntry) -> Result<Box<dyn Read>, PassError> {
        if self.flip && f.path.as_str() == "data.txt" {
            let n = self.opens.get();
            self.opens.set(n + 1);
            if n >= 1 {
                return Ok(Box::new(Cursor::new(b"other bytes".to_vec())));
            }
        }
        self.inner.open(f)
    }
}

fn flaky(t: &tempfile::TempDir, flip: bool) -> Flaky {
    Flaky {
        inner: FolderSource::open(t.path()).unwrap(),
        opens: Cell::new(0),
        flip,
    }
}

#[test]
fn a_file_that_changes_between_check_and_copy_stops_the_build_and_leaves_nothing() {
    let t = scratch(false);
    put(&t, "files/data.txt", b"first bytes");
    let out = t.path().join("_o");
    let e = build(&flaky(&t, true), &out, &opts()).expect_err("must stop");
    assert!(e.to_string().contains("changed while the build ran"), "{e}");
    assert!(!out.exists(), "nothing may be left behind");
    // counter-probe: the same source that stays the same builds
    let out = t.path().join("_o2");
    build(&flaky(&t, false), &out, &opts()).expect("builds");
    assert_eq!(std::fs::read(out.join("data.txt")).unwrap(), b"first bytes");
}
