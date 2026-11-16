//! Overview pages of folders without an index, dead links and anchors, and what `[build]` sets.
mod common;
use common::*;
use fs_text_site::{build, FolderSource};
use std::fs;

fn append(t: &tempfile::TempDir, more: &str) {
    let p = t.path().join("site.toml");
    let mut s = fs::read_to_string(&p).unwrap();
    s.push_str(more);
    fs::write(p, s).unwrap();
}

fn run(t: &tempfile::TempDir) -> (std::path::PathBuf, fs_text_site::BuildReport) {
    let out = t.path().join("_o");
    let src = FolderSource::open(t.path()).unwrap();
    let r = build(&src, &out, &opts()).expect("builds");
    (out, r)
}

fn page(out: &std::path::Path, p: &str) -> String {
    fs::read_to_string(out.join(p)).unwrap()
}

// ---- overview pages ---------------------------------------------------------------------

#[test]
fn a_folder_of_72_pages_without_index_gets_an_overview_listing_all() {
    let t = scratch(false);
    for i in 0..72 {
        put(
            &t,
            &format!("content/en/fields/f{i:02}.md"),
            format!("# Field {i}\n").as_bytes(),
        );
    }
    put(
        &t,
        "content/en/index.md",
        b"# Hi\n\nSee [fields](fields/).\n",
    );
    let (out, r) = run(&t);
    assert_eq!(r.dead_links, 0, "{}", r.to_text());
    let h = page(&out, "en/fields/index.html");
    for i in 0..72 {
        assert!(
            h.contains(&format!("href=\"/en/fields/f{i:02}/\">Field {i}</a>")),
            "{i}"
        );
    }
    assert!(
        h.contains("Automatically generated overview"),
        "marked as generated"
    );
    assert_eq!(
        h.matches("class=\"more\"").count(),
        1,
        "only the side navigation is cut"
    );
    assert!(out.join("sitemap.xml").exists());
    assert!(page(&out, "sitemap.xml").contains("/en/fields/</loc>"));
}

#[test]
fn a_hand_written_index_wins_and_a_nested_gap_is_filled() {
    let t = scratch(false);
    put(&t, "content/en/a/index.md", b"# Mine\n\nhand written\n");
    put(&t, "content/en/a/b/c/leaf.md", b"# Leaf\n");
    let (out, _) = run(&t);
    let a = page(&out, "en/a/index.html");
    assert!(a.contains("hand written") && !a.contains("Automatically generated"));
    let b = page(&out, "en/a/b/index.html");
    assert!(b.contains("Automatically generated") && b.contains("href=\"/en/a/b/c/\""));
    assert!(page(&out, "en/a/b/c/index.html").contains("href=\"/en/a/b/c/leaf/\">Leaf</a>"));
}

#[test]
fn german_overview_is_german_and_the_order_is_fixed() {
    let t = scratch(false);
    fs::write(
        t.path().join("site.toml"),
        fs::read_to_string(t.path().join("site.toml"))
            .unwrap()
            .replace("[\"en\"]", "[\"de\"]"),
    )
    .unwrap();
    fs::rename(t.path().join("content/en"), t.path().join("content/de")).unwrap();
    for n in ["b10", "b2", "a"] {
        put(
            &t,
            &format!("content/de/x/{n}.md"),
            format!("# T {n}\n").as_bytes(),
        );
    }
    let (out, _) = run(&t);
    let h = page(&out, "de/x/index.html");
    assert!(h.contains("Automatisch erzeugte"));
    let pos = |s: &str| h.find(s).unwrap();
    assert!(pos("T a") < pos("T b2") && pos("T b2") < pos("T b10"));
}

#[test]
fn switched_off_the_folder_gets_no_page_and_its_links_are_dead() {
    let t = scratch(false);
    put(&t, "content/en/f/x.md", b"# X\n");
    put(&t, "content/en/index.md", b"# Hi\n\n[folder](f/)\n");
    append(&t, "[build]\nfolder_index = false\n");
    let e = refused(&t);
    assert!(e.contains("f/"), "{e}");
    let t = scratch(false);
    put(&t, "content/en/f/x.md", b"# X\n");
    put(&t, "content/en/index.md", b"# Hi\n\n[folder](f/)\n");
    assert!(builds(&t), "counter-probe: on by default");
}

// ---- dead links -------------------------------------------------------------------------

#[test]
fn a_dead_link_stops_the_build_by_default() {
    let t = scratch(false);
    put(&t, "content/en/index.md", b"# Hi\n\n[x](nowhere.md)\n");
    assert!(refused(&t).contains("nowhere.md"));
}

#[test]
fn a_dead_anchor_is_a_dead_link_too() {
    let t = scratch(false);
    put(&t, "content/en/a.md", b"# A\n\n## Real\n");
    put(
        &t,
        "content/en/index.md",
        b"# Hi\n\n[ok](a.md#real) [bad](a.md#nope) [own](#hi) [badown](#zzz)\n",
    );
    let e = refused(&t);
    assert!(e.contains("nope") && e.contains("zzz"), "{e}");
    assert!(!e.contains("#real") && !e.contains("\"#hi\""), "{e}");
}

#[test]
fn list_mode_builds_lists_all_and_counts() {
    let t = scratch(false);
    put(
        &t,
        "content/en/index.md",
        b"# Hi\n\n[x](nowhere.md) [y](nothing.md) [z](a.md#gone)\n",
    );
    put(&t, "content/en/a.md", b"# A\n");
    append(&t, "[build]\ndead_links = \"list\"\n");
    let (out, r) = run(&t);
    assert_eq!(r.dead_links, 3, "{}", r.to_text());
    let text = r.to_text();
    assert!(text.contains("nowhere.md") && text.contains("nothing.md") && text.contains("gone"));
    assert!(text.contains("3 dead links"));
    assert!(r.findings.iter().all(|f| !f.fatal));
    assert!(page(&out, "site-manifest.json").contains("\"dead_links\": 3"));
}

#[test]
fn a_clean_site_says_zero_and_external_links_are_not_judged() {
    let t = scratch(false);
    put(
        &t,
        "content/en/index.md",
        b"# Hi\n\n[e](https://elsewhere.test/x#y)\n",
    );
    let (out, r) = run(&t);
    assert_eq!(r.dead_links, 0);
    assert!(page(&out, "site-manifest.json").contains("\"dead_links\": 0"));
}

#[test]
fn bad_build_values_are_refused() {
    let t = scratch(false);
    append(&t, "[build]\ndead_links = \"ignore\"\n");
    assert!(fs_text_site::FolderSource::open(t.path()).is_err());
}

// ---- sitemap ----------------------------------------------------------------------------

#[test]
fn a_page_with_one_language_has_the_same_hreflang_in_html_and_sitemap() {
    let t = scratch(false);
    put(&t, "content/en/only.md", b"# Only\n");
    let (out, _) = run(&t);
    let sm = page(&out, "sitemap.xml");
    let html = page(&out, "en/only/index.html");
    for needle in [
        "hreflang=\"en\" href=\"https://example.test/en/only/\"",
        "hreflang=\"x-default\" href=\"https://example.test/\"",
    ] {
        assert!(html.contains(needle), "html {needle}");
        assert!(sm.contains(needle), "sitemap {needle}");
    }
}
