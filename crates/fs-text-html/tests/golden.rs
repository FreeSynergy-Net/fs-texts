//! Golden files: fixture Markdown to the expected HTML, byte for byte.
//! Write new expectations only with `UPDATE_GOLDEN=1` and look at the diff.
mod common;
mod frame_fixture {
    include!("common/frame_fixture.rs");
}
use common::page;
use fs_text_html::render_page;
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn check(name: &str, got: &str) {
    let path = dir().join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, got).expect("write golden");
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing golden {name}"));
    assert_eq!(got, want, "golden {name} differs (UPDATE_GOLDEN=1 to rewrite, then review)");
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(dir().join(name)).expect("fixture")
}

#[test]
fn rich_fragment() {
    check("rich.html", &page(&fixture("rich.md")).body);
}

#[test]
fn links_fragment() {
    check("links.html", &page(&fixture("links.md")).body);
}

#[test]
fn full_page() {
    let p = page(&fixture("rich.md"));
    check("page.html", &render_page(&frame_fixture::frame(), &p));
}
