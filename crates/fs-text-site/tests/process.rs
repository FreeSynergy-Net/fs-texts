//! Two separate processes build the same sources: the folders are byte-equal (also modes and
//! times); the hreflang values of the pages are the exact set of existing variants.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/multi")
}

fn run(out: &Path, mtime: &str) {
    let o = Command::new(env!("CARGO_BIN_EXE_fs-text-site"))
        .args(["build-all"])
        .arg(fixtures())
        .arg(out)
        .env("SITE_MTIME", mtime)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

fn tree(root: &Path) -> BTreeMap<String, (Vec<u8>, u64, u32)> {
    use std::os::unix::fs::PermissionsExt;
    fn walk(root: &Path, d: &Path, o: &mut BTreeMap<String, (Vec<u8>, u64, u32)>) {
        for e in fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            let m = fs::symlink_metadata(&p).unwrap();
            let t = m
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().to_string();
            let mode = m.permissions().mode() & 0o777;
            if m.is_dir() {
                o.insert(format!("{rel}/"), (vec![], t, mode));
                walk(root, &p, o);
            } else {
                o.insert(rel, (fs::read(&p).unwrap(), t, mode));
            }
        }
    }
    let mut o = BTreeMap::new();
    walk(root, root, &mut o);
    o
}

#[test]
fn two_processes_build_byte_equal_folders() {
    let t = tempfile::tempdir().unwrap();
    run(&t.path().join("a"), "1700000000");
    run(&t.path().join("b"), "1700000000");
    let (a, b) = (tree(&t.path().join("a")), tree(&t.path().join("b")));
    assert!(a.len() > 30);
    assert_eq!(a, b);
}

#[test]
fn a_run_without_a_release_time_refuses() {
    let t = tempfile::tempdir().unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_fs-text-site"))
        .arg("build-all")
        .arg(fixtures())
        .arg(t.path().join("o"))
        .env_remove("SITE_MTIME")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(!t.path().join("o").exists());
}

fn alternates(html: &str) -> Vec<(String, String)> {
    html.lines()
        .filter(|l| l.starts_with("<link rel=\"alternate\""))
        .map(|l| {
            let g = |k: &str| {
                let s = l.split(&format!("{k}=\"")).nth(1).unwrap();
                s.split('"').next().unwrap().to_string()
            };
            (g("hreflang"), g("href"))
        })
        .collect()
}

#[test]
fn hreflang_lists_exactly_the_existing_variants_with_absolute_addresses() {
    let t = tempfile::tempdir().unwrap();
    run(&t.path().join("o"), "1700000000");
    let o = t.path().join("o");
    let p = |f: &str| alternates(&fs::read_to_string(o.join(f)).unwrap());
    let a = |l: &str, h: &str| (l.to_string(), h.to_string());
    // both variants exist
    assert_eq!(
        p("wiki/de/index.html"),
        vec![
            a("de", "https://example.test/wiki/de/"),
            a("en", "https://example.test/wiki/en/"),
            a("x-default", "https://example.test/wiki/"),
        ]
    );
    // only one variant exists: itself and x-default only
    assert_eq!(
        p("wiki/en/guide/only-en/index.html"),
        vec![
            a("en", "https://example.test/wiki/en/guide/only-en/"),
            a("x-default", "https://example.test/wiki/"),
        ]
    );
    // the other site of the same host has its own set and its own x-default
    assert_eq!(
        p("landing/de/about/index.html"),
        vec![
            a("de", "https://example.test/de/about/"),
            a("x-default", "https://example.test/"),
        ]
    );
    // the contents pages link each other across languages
    assert_eq!(
        p("wiki/de/inhalt/index.html"),
        vec![
            a("de", "https://example.test/wiki/de/inhalt/"),
            a("en", "https://example.test/wiki/en/contents/"),
            a("x-default", "https://example.test/wiki/"),
        ]
    );
}
