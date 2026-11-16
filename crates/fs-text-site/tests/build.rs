//! The builder on fixture sites: golden files per site, byte equality of passed files, determinism.
mod common;
use common::*;
use fs_text_site::{build, build_all, FolderSource};
use sha2::{Digest, Sha256};
use std::fs;
use std::time::{Duration, UNIX_EPOCH};

#[test]
fn golden_landing_and_wiki() {
    let t = tempfile::tempdir().unwrap();
    let out = t.path().join("out");
    build_multi(&out);
    check_golden(
        "landing",
        &out.join("landing"),
        &[
            "de/index.html",
            "de/about/index.html",
            "de/inhalt/index.html",
            "en/index.html",
            "sitemap.xml",
            "robots.txt",
            "404.html",
            "site.toml",
            "site-manifest.json",
        ],
    );
    check_golden(
        "wiki",
        &out.join("wiki"),
        &[
            "index.html",
            "de/index.html",
            "de/guide/index.html",
            "de/guide/install/index.html",
            "de/inhalt/index.html",
            "en/guide/only-en/index.html",
            "en/contents/index.html",
            "sitemap.xml",
            "404.html",
            "site.toml",
            "site-manifest.json",
        ],
    );
    // a site below a path has no robots.txt (it would not be read there)
    assert!(!out.join("wiki/robots.txt").exists());
    // the user's index.html replaced the generated language choice
    assert!(fs::read_to_string(out.join("landing/index.html"))
        .unwrap()
        .contains("Von Hand"));
}

#[test]
fn each_site_has_its_own_folder_navigation_sitemap_and_hreflang() {
    let t = tempfile::tempdir().unwrap();
    let out = t.path().join("out");
    build_multi(&out);
    let wiki = fs::read_to_string(out.join("wiki/de/guide/index.html")).unwrap();
    assert!(wiki.contains("https://example.test/wiki/de/guide/"));
    assert!(wiki.contains("hreflang=\"x-default\" href=\"https://example.test/wiki/\""));
    assert!(!wiki.contains("/landing"));
    let map = fs::read_to_string(out.join("wiki/sitemap.xml")).unwrap();
    assert!(map.contains("https://example.test/wiki/en/guide/only-en/"));
    assert!(!map.contains("Willkommen"));
    let lmap = fs::read_to_string(out.join("landing/sitemap.xml")).unwrap();
    assert!(!lmap.contains("/wiki/"));
    // only-en has no German variant: no language bar link and no de alternate for it
    let only = fs::read_to_string(out.join("wiki/en/guide/only-en/index.html")).unwrap();
    assert!(!only.contains("hreflang=\"de\""));
    // the link to a page that exists only in English falls back to the default language
    assert!(wiki.contains("href=\"/wiki/en/guide/only-en/\""));
}

#[test]
fn passed_files_are_byte_equal_and_listed_in_the_manifest() {
    let t = tempfile::tempdir().unwrap();
    let out = t.path().join("out");
    build_multi(&out);
    let src = fixtures().join("multi/landing/files");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("landing/site-manifest.json")).unwrap()).unwrap();
    for rel in [
        "index.html",
        "app.js",
        "css/site.css",
        "img/logo.png",
        ".well-known/security.txt",
    ] {
        let a = fs::read(src.join(rel)).unwrap();
        let b = fs::read(out.join("landing").join(rel)).unwrap();
        assert_eq!(a, b, "{rel}");
        let e = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["path"] == rel)
            .unwrap();
        assert_eq!(e["sha256"], format!("{:x}", Sha256::digest(&a)));
        assert_eq!(e["kind"], "passthrough");
        assert_eq!(e["active"], rel.ends_with(".html") || rel.ends_with(".js"));
        assert_eq!(e["bytes"], a.len());
    }
    // every file in the folder (but the manifest) is in the manifest, with the right hash
    let snap = snapshot(&out.join("landing"));
    let listed: Vec<&str> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["path"].as_str().unwrap())
        .collect();
    for (p, (bytes, _, _)) in &snap {
        if p.ends_with('/') || p == "site-manifest.json" {
            continue;
        }
        assert!(listed.contains(&p.as_str()), "{p} missing in manifest");
        let e = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["path"] == p.as_str())
            .unwrap();
        assert_eq!(e["sha256"], format!("{:x}", Sha256::digest(bytes)));
    }
    // the passed HTML carries no frame, no meta, no CSP
    let html = fs::read_to_string(out.join("landing/index.html")).unwrap();
    assert!(!html.contains("Content-Security-Policy"));
}

#[test]
fn building_twice_gives_the_same_folder_bytes_modes_and_times() {
    let t = tempfile::tempdir().unwrap();
    build_multi(&t.path().join("a"));
    build_multi(&t.path().join("b"));
    let (a, b) = (snapshot(&t.path().join("a")), snapshot(&t.path().join("b")));
    assert_eq!(a, b);
    // the time comes from the option, not from the clock
    assert!(a.values().all(|(_, secs, _)| *secs == 1_700_000_000));
    // another release time changes the times, nothing else
    let mut o = opts();
    o.mtime = UNIX_EPOCH + Duration::from_secs(1_700_000_500);
    build_all(&fixtures().join("multi"), &t.path().join("c"), &o).unwrap();
    let c = snapshot(&t.path().join("c"));
    assert_eq!(a.keys().collect::<Vec<_>>(), c.keys().collect::<Vec<_>>());
    for (k, v) in &a {
        assert_eq!(v.0, c[k].0, "{k}");
        assert_eq!(c[k].1, 1_700_000_500);
    }
}

#[test]
fn media_are_named_by_hash_and_unused_media_are_not_published() {
    let t = tempfile::tempdir().unwrap();
    let out = t.path().join("out");
    build_multi(&out);
    let media: Vec<_> = fs::read_dir(out.join("wiki/_fs/media")).unwrap().collect();
    assert_eq!(media.len(), 1, "one file, used twice");
    let name = media[0]
        .as_ref()
        .unwrap()
        .file_name()
        .into_string()
        .unwrap();
    assert!(name.ends_with(".png") && name.len() == 16 + 4);
    let page = fs::read_to_string(out.join("wiki/de/index.html")).unwrap();
    assert!(page.contains(&format!("/wiki/_fs/media/{name}")));
}

#[test]
fn legacy_schema_1_site_builds_with_defaults() {
    let t = tempfile::tempdir().unwrap();
    let src = FolderSource::open(fixtures().join("legacy")).unwrap();
    let r = build(&src, &t.path().join("o"), &opts()).unwrap();
    assert!(r.pages >= 2);
    check_golden(
        "legacy",
        &t.path().join("o"),
        &["en/index.html", "site.toml"],
    );
}
