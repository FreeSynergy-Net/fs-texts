//! Helpers shared by the build tests.
#![allow(dead_code)]
use fs_text_site::{build, build_all, BuildOptions, FolderSource, SiteError};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

pub fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
pub fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}
pub fn opts() -> BuildOptions {
    BuildOptions::new(UNIX_EPOCH + Duration::from_secs(1_700_000_000))
}

/// path -> (bytes, mtime secs, mode) of a whole folder.
pub fn snapshot(root: &Path) -> BTreeMap<String, (Vec<u8>, u64, u32)> {
    use std::os::unix::fs::PermissionsExt;
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, (Vec<u8>, u64, u32)>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().to_string();
            let m = fs::symlink_metadata(&p).unwrap();
            let t = m
                .modified()
                .unwrap()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let mode = m.permissions().mode() & 0o777;
            if m.is_dir() {
                out.insert(format!("{rel}/"), (Vec::new(), t, mode));
                walk(root, &p, out);
            } else {
                out.insert(rel, (fs::read(&p).unwrap(), t, mode));
            }
        }
    }
    let mut o = BTreeMap::new();
    walk(root, root, &mut o);
    o
}

pub fn check_golden(site: &str, out: &Path, files: &[&str]) {
    for f in files {
        let got = fs::read_to_string(out.join(f)).unwrap();
        let g = golden().join(site).join(f);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            fs::create_dir_all(g.parent().unwrap()).unwrap();
            fs::write(&g, &got).unwrap();
        }
        let want = fs::read_to_string(&g).unwrap_or_else(|_| panic!("missing golden {site}/{f}"));
        assert_eq!(
            got, want,
            "golden {site}/{f} (UPDATE_GOLDEN=1, then review)"
        );
    }
}

pub fn build_multi(out: &Path) {
    let r = build_all(&fixtures().join("multi"), out, &opts()).expect("multi builds");
    assert_eq!(r.len(), 2);
}

// ---- refusals -------------------------------------------------------------------------

/// A scratch site with `site.toml` v2 (passthrough mode unless `static_mode`).
pub fn scratch(static_mode: bool) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let mode = if static_mode { "static" } else { "passthrough" };
    fs::write(
        t.path().join("site.toml"),
        format!("schema = 2\npurpose = \"website\"\n[site]\nid = \"s\"\nlanguages = [\"en\"]\n[names]\nprimary = \"example.test\"\n[acme]\nmethod = \"http-01\"\n[csp]\nmode = \"{mode}\"\n"),
    )
    .unwrap();
    fs::create_dir_all(t.path().join("content/en")).unwrap();
    fs::write(t.path().join("content/en/index.md"), "# Hi\n").unwrap();
    fs::create_dir_all(t.path().join("files")).unwrap();
    t
}

pub fn put(t: &tempfile::TempDir, rel: &str, bytes: &[u8]) {
    let p = t.path().join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

pub fn refused(t: &tempfile::TempDir) -> String {
    let out = t.path().join("_out");
    let src = FolderSource::open(t.path()).unwrap();
    let e = build(&src, &out, &opts()).expect_err("must be refused");
    assert!(!out.exists(), "nothing may be left behind after a refusal");
    e.to_string()
}

pub fn builds(t: &tempfile::TempDir) -> bool {
    let src = FolderSource::open(t.path()).unwrap();
    build(&src, &t.path().join("_out2"), &opts()).is_ok()
}

pub fn put_at(p: &Path, b: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, b).unwrap();
}

pub fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let p = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &p);
        } else {
            fs::copy(e.path(), p).unwrap();
        }
    }
}

#[test]
pub fn colliding_sites_stop_before_anything_is_written() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("sites");
    copy_tree(&fixtures().join("multi/landing"), &root.join("one"));
    copy_tree(&fixtures().join("multi/landing"), &root.join("two"));
    let out = t.path().join("o");
    let e = build_all(&root, &out, &opts()).unwrap_err();
    assert!(matches!(e, SiteError::Config(_)));
    assert!(!out.exists());
}
