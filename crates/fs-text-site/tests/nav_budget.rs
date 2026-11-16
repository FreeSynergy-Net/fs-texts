//! The links to the contents page and the sitemap are on every page, whatever the size of the
//! tree; where the navigation was cut, a hint leads to the contents page.
mod common;
use common::*;
use fs_text_site::{build, FolderSource};
use std::fs;

fn built(t: &tempfile::TempDir) -> std::path::PathBuf {
    let out = t.path().join("_o");
    let src = FolderSource::open(t.path()).unwrap();
    build(&src, &out, &opts()).expect("builds");
    out
}

#[test]
fn contents_link_survives_45_top_level_pages() {
    let t = scratch(false);
    for i in 0..45 {
        put(
            &t,
            &format!("content/en/p{i:02}.md"),
            format!("# P{i}\n").as_bytes(),
        );
    }
    let out = built(&t);
    for page in ["en/index.html", "en/p00/index.html", "en/p44/index.html"] {
        let h = fs::read_to_string(out.join(page)).unwrap();
        assert!(h.contains("href=\"/en/contents/\""), "{page}");
        assert!(h.contains("href=\"/sitemap.xml\""), "{page}");
        assert!(h.contains("class=\"more\""), "{page}: cut without a hint");
    }
    let c = fs::read_to_string(out.join("en/contents/index.html")).unwrap();
    assert!(
        c.contains("href=\"/en/p44/\""),
        "the contents page lists all"
    );
}

#[test]
fn a_flat_folder_of_72_pages_keeps_the_links_and_hints() {
    let t = scratch(false);
    for i in 0..72 {
        put(
            &t,
            &format!("content/en/fields/f{i:02}.md"),
            format!("# F{i}\n").as_bytes(),
        );
    }
    let out = built(&t);
    let h = fs::read_to_string(out.join("en/fields/f70/index.html")).unwrap();
    assert!(h.contains("href=\"/en/contents/\""));
    assert!(h.contains("class=\"more\""));
    assert!(h.contains("aria-current=\"page\""));
}

#[test]
fn a_small_site_has_the_links_but_no_hint() {
    let t = scratch(false);
    let out = built(&t);
    let h = fs::read_to_string(out.join("en/index.html")).unwrap();
    assert!(h.contains("href=\"/en/contents/\"") && h.contains("href=\"/sitemap.xml\""));
    assert!(!h.contains("class=\"more\""));
}
