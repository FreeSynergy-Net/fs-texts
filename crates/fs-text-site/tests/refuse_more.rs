//! More refusals: nested sites and per-folder rules (continued from `refuse.rs`).
mod common;
use common::*;
use fs_text_site::build_all;
use std::fs;

#[test]
fn nested_sites_reserve_their_prefix_against_passed_files() {
    // landing at / and wiki at /wiki: a file files/wiki/x.css in landing is refused
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("sites");
    fs::create_dir_all(&root).unwrap();
    for n in ["landing", "wiki"] {
        let from = fixtures().join("multi").join(n);
        let to = root.join(n);
        copy_tree(&from, &to);
    }
    put_at(&root.join("landing/files/wiki/x.css"), b"a{}");
    let e = build_all(&root, &t.path().join("o"), &opts())
        .unwrap_err()
        .to_string();
    assert!(e.contains("generated prefix"), "{e}");
}

/// The area rule decides per folder: an active file is fine under an area with `passthrough`
/// and refused beside it (mutation probe: with the area removed the same file is refused).
#[test]
fn csp_area_allows_active_files_only_in_its_folder() {
    let site = |area: bool| {
        let t = scratch(true);
        if area {
            let mut s = fs::read_to_string(t.path().join("site.toml")).unwrap();
            s.push_str("[[csp.area]]\npath = \"/apps\"\nmode = \"passthrough\"\n");
            fs::write(t.path().join("site.toml"), s).unwrap();
        }
        t
    };
    let t = site(true);
    put(&t, "files/apps/a.html", b"<script>1</script>");
    assert!(builds(&t));
    let t = site(true);
    put(&t, "files/apps/a.html", b"<script>1</script>");
    put(&t, "files/other.html", b"<script>1</script>");
    assert!(refused(&t).contains("other.html"));
    let t = site(false);
    put(&t, "files/apps/a.html", b"<script>1</script>");
    assert!(refused(&t).contains("apps/a.html"));
}
