//! Two sites of the same name on different domains (a landing page and a wiki, both "web").
mod common;
use common::*;
use fs_text_site::build_all;
use std::fs;

fn site(root: &std::path::Path, dir: &str, host: &str, prefix: &str) {
    let toml = format!("schema = 2\npurpose = \"website\"\n[site]\nid = \"web\"\npath_prefix = \"{prefix}\"\nlanguages = [\"en\"]\n[names]\nprimary = \"{host}\"\n[acme]\nmethod = \"http-01\"\n");
    put_at(&root.join(dir).join("site.toml"), toml.as_bytes());
    put_at(
        &root.join(dir).join("content/en/index.md"),
        format!("# {host}\n").as_bytes(),
    );
}

#[test]
fn two_sites_named_alike_on_two_domains_build_into_two_folders() {
    let t = tempfile::tempdir().unwrap();
    site(t.path(), "a", "example.test", "/");
    site(t.path(), "b", "wiki.example.test", "/");
    let out = t.path().join("out");
    let r = build_all(t.path(), &out, &opts()).expect("builds");
    assert_eq!(r.len(), 2);
    let a = fs::read_to_string(out.join("example.test/en/index.html")).unwrap();
    let b = fs::read_to_string(out.join("wiki.example.test/en/index.html")).unwrap();
    assert!(a.contains("https://example.test/en/") && !a.contains("wiki.example.test"));
    assert!(b.contains("https://wiki.example.test/en/"));
}

#[test]
fn the_same_name_on_the_same_domain_and_path_still_stops() {
    let t = tempfile::tempdir().unwrap();
    site(t.path(), "a", "example.test", "/");
    site(t.path(), "b", "example.test", "/");
    assert!(build_all(t.path(), &t.path().join("out"), &opts()).is_err());
}
