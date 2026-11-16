//! Sites of other domains in one run: one folder per id; the same id is refused before anything is written.
mod common;
use common::*;
use fs_text_site::build_all;
use std::fs;

fn site(root: &std::path::Path, dir: &str, id: &str, host: &str, prefix: &str) {
    let toml = format!("schema = 2\npurpose = \"website\"\n[site]\nid = \"{id}\"\npath_prefix = \"{prefix}\"\nlanguages = [\"en\"]\n[names]\nprimary = \"{host}\"\n[acme]\nmethod = \"http-01\"\n");
    put_at(&root.join(dir).join("site.toml"), toml.as_bytes());
    put_at(
        &root.join(dir).join("content/en/index.md"),
        format!("# {host}\n").as_bytes(),
    );
}

#[test]
fn sites_of_other_domains_build_into_one_folder_per_id() {
    let t = tempfile::tempdir().unwrap();
    site(t.path(), "a", "main", "example.test", "/");
    site(t.path(), "b", "wiki", "wiki.example.test", "/");
    let out = t.path().join("out");
    let r = build_all(t.path(), &out, &opts()).expect("builds");
    assert_eq!(r.len(), 2);
    let a = fs::read_to_string(out.join("main/en/index.html")).unwrap();
    let b = fs::read_to_string(out.join("wiki/en/index.html")).unwrap();
    assert!(a.contains("https://example.test/en/") && !a.contains("wiki.example.test"));
    assert!(b.contains("https://wiki.example.test/en/"));
}

fn refused_untouched(t: &tempfile::TempDir) -> String {
    let out = t.path().join("out");
    let e = build_all(t.path(), &out, &opts()).expect_err("must stop");
    assert!(!out.exists(), "nothing may be written before the check");
    e.to_string()
}

#[test]
fn the_same_id_is_refused_before_anything_is_written() {
    // on another host, on the same host with prefixes `/b/c` and `/b_c` (which an output folder
    // named after host and prefix would have joined into one), and on the same path
    for (h1, p1, h2, p2) in [
        ("example.test", "/", "wiki.example.test", "/"),
        ("example.test", "/b/c", "example.test", "/b_c"),
        ("example.test", "/", "example.test", "/"),
    ] {
        let t = tempfile::tempdir().unwrap();
        site(t.path(), "a", "web", h1, p1);
        site(t.path(), "b", "web", h2, p2);
        let e = refused_untouched(&t);
        assert!(e.contains("two sites have the id \"web\""), "{e}");
    }
    // counter-probe: with different ids the prefixes `/b/c` and `/b_c` build into two folders
    let t = tempfile::tempdir().unwrap();
    site(t.path(), "a", "one", "example.test", "/b/c");
    site(t.path(), "b", "two", "example.test", "/b_c");
    let out = t.path().join("out");
    build_all(t.path(), &out, &opts()).expect("builds");
    assert!(out.join("one/en/index.html").is_file() && out.join("two/en/index.html").is_file());
}
