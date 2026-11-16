//! The builder refuses: every rule of the pass-through, the paths and the sites, with counter-probes.
mod common;
use common::*;
use fs_text_site::{build, build_all, FolderSource, Gate, SiteError};
use std::fs;

#[test]
fn the_scratch_site_itself_builds() {
    let t = scratch(false);
    assert!(builds(&t));
}

#[test]
fn page_addresses_are_never_rewritten_silently() {
    let t = scratch(false);
    put(&t, "content/en/Upper.md", b"# x\n");
    assert!(refused(&t).contains("lower case"));
}

#[test]
fn a_page_twice_is_refused() {
    let t = scratch(false);
    put(&t, "content/en/a.md", b"# a\n");
    put(&t, "content/en/a/index.md", b"# a2\n");
    assert!(refused(&t).contains("twice"));
}

#[test]
fn the_contents_address_is_reserved() {
    let t = scratch(false);
    put(&t, "content/en/contents.md", b"# c\n");
    assert!(refused(&t).contains("contents page"));
}

#[test]
fn pass_files_may_not_use_generated_names() {
    for p in [
        "en/x.html",
        "_fs/x.css",
        "sitemap.xml",
        "site-manifest.json",
        "site.toml",
    ] {
        let t = scratch(false);
        put(&t, &format!("files/{p}"), b"<p>x</p>");
        assert!(refused(&t).contains("generated prefix"), "{p}");
    }
}

#[test]
fn letter_case_twins_and_bad_paths_are_refused() {
    let t = scratch(false);
    put(&t, "files/a.css", b"a{}");
    put(&t, "files/A.css", b"a{}");
    assert!(refused(&t).contains("twice"));
    let t = scratch(false);
    put(&t, "files/sp ace.css", b"a{}");
    assert!(refused(&t).contains("not allowed in a path"));
    let t = scratch(false);
    put(&t, "files/.hidden/x.css", b"a{}");
    assert!(refused(&t).contains("hidden"));
}

#[test]
fn well_known_allows_only_security_txt() {
    let t = scratch(false);
    put(&t, "files/.well-known/acme-challenge/x.txt", b"x");
    assert!(refused(&t).contains("generated prefix"));
    let t = scratch(false);
    put(&t, "files/.well-known/other.txt", b"x");
    assert!(refused(&t).contains("security.txt"));
    let t = scratch(false);
    put(
        &t,
        "files/.well-known/security.txt",
        b"Contact: mailto:a@example.test\n",
    );
    assert!(builds(&t));
}

#[test]
fn a_symlink_and_a_hard_link_are_refused() {
    use std::os::unix::fs::symlink;
    let t = scratch(false);
    put(&t, "outside.css", b"a{}");
    symlink(t.path().join("outside.css"), t.path().join("files/l.css")).unwrap();
    assert!(refused(&t).contains("symbolic link"));
    let t = scratch(false);
    put(&t, "files/a.css", b"a{}");
    fs::hard_link(t.path().join("files/a.css"), t.path().join("files/b.css")).unwrap();
    assert!(refused(&t).contains("hard link"));
    // a link in a folder of the path
    let t = scratch(false);
    put(&t, "real/x.css", b"a{}");
    symlink(t.path().join("real"), t.path().join("files/dir")).unwrap();
    assert!(refused(&t).contains("symbolic link"));
}

#[test]
fn content_must_fit_the_extension_and_types_must_be_known() {
    let t = scratch(false);
    put(&t, "files/a.png", b"<html>not a png</html>");
    assert!(refused(&t).contains("does not start like"));
    let t = scratch(false);
    put(&t, "files/a.bin", b"x");
    assert!(refused(&t).contains("no media type"));
    let t = scratch(false);
    put(&t, "files/a.css", b"\xff\xfe");
    assert!(refused(&t).contains("UTF-8"));
}

#[test]
fn too_big_files_are_refused() {
    let t = scratch(false);
    put(&t, "files/a.txt", &vec![b'a'; 2000]);
    let mut o = opts();
    o.max_file_bytes = 1000;
    let src = FolderSource::open(t.path()).unwrap();
    let e = build(&src, &t.path().join("_o"), &o)
        .unwrap_err()
        .to_string();
    assert!(e.contains("bigger than the limit"), "{e}");
}

/// Mutation probe: the rule "an active file needs csp.mode passthrough" is what refuses here;
/// with the mode switched to passthrough the same folder builds.
#[test]
fn an_active_file_in_a_static_site_is_refused_and_in_passthrough_mode_it_builds() {
    for p in ["files/a.html", "files/a.js", "files/a.svg"] {
        let t = scratch(true);
        put(&t, p, b"<svg></svg><script>1</script>");
        assert!(refused(&t).contains("csp.mode"), "{p}");
    }
    let t = scratch(false);
    put(&t, "files/a.html", b"<script>alert(1)</script>");
    assert!(builds(&t));
    // never a static site with only quiet files: builds
    let t = scratch(true);
    put(&t, "files/a.css", b"a{}");
    assert!(builds(&t));
}

#[test]
fn page_media_may_not_run_anything() {
    for p in [
        "content/en/a.html",
        "content/en/a.svg",
        "content/en/a.js",
        "content/en/a.css",
    ] {
        let t = scratch(false);
        put(&t, p, b"<svg></svg>");
        assert!(refused(&t).contains("page media"), "{p}");
    }
}

#[test]
fn a_language_folder_that_site_toml_does_not_list_is_refused() {
    let t = scratch(false);
    put(&t, "content/fr/index.md", b"# Salut\n");
    assert!(refused(&t).contains("not one of the languages"));
}

#[test]
fn the_output_folder_is_never_overwritten() {
    let t = scratch(false);
    let out = t.path().join("o");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("keep.txt"), "mine").unwrap();
    let src = FolderSource::open(t.path()).unwrap();
    let e = build(&src, &out, &opts()).unwrap_err();
    assert!(matches!(e, SiteError::OutputNotEmpty(_)));
    assert_eq!(fs::read_to_string(out.join("keep.txt")).unwrap(), "mine");
}

#[test]
fn a_missing_image_stops_the_build_but_a_dead_link_only_reports() {
    let t = scratch(false);
    put(&t, "content/en/index.md", b"# Hi\n\n![x](nope.png)\n");
    assert!(refused(&t).contains("missing"));
    let t = scratch(false);
    put(&t, "content/en/index.md", b"# Hi\n\n[x](nope.md)\n");
    let src = FolderSource::open(t.path()).unwrap();
    let r = build(&src, &t.path().join("_o"), &opts()).unwrap();
    assert!(r
        .findings
        .iter()
        .any(|f| f.message.contains("points to nothing") && !f.fatal));
    // the gate turns it into a stop
    let mut o = opts();
    o.policy.dead_links = Gate::Fail;
    assert!(build(&src, &t.path().join("_o2"), &o).is_err());
}

#[test]
fn link_escapes_do_not_leave_the_content_tree() {
    let t = scratch(false);
    put(&t, "content/en/index.md", b"# Hi\n\n[a](../../etc/passwd) [b](/../x.md) [c](..\\x.md) [d](%2e%2e/x.md) [e](javascript:alert(1))\n");
    let src = FolderSource::open(t.path()).unwrap();
    let r = build(&src, &t.path().join("_o"), &opts()).unwrap();
    let html = fs::read_to_string(t.path().join("_o/en/index.html")).unwrap();
    assert!(
        !html.contains("<a href=\"../") && !html.contains("javascript:") && !html.contains("%2e")
    );
    assert!(r.findings.len() >= 4);
}

#[test]
fn a_declared_checksum_that_does_not_match_is_refused() {
    use fs_text_html::pass::{PassError, PassPath};
    use fs_text_site::{Area, FileEntry, Listing, PageEntry, SiteConfig, SiteSource};
    struct Lying(SiteConfig, fs_text::Document);
    impl SiteSource for Lying {
        fn config(&self) -> &SiteConfig {
            &self.0
        }
        fn pages(&self) -> Listing<PageEntry> {
            Listing {
                items: vec![PageEntry {
                    lang: self.0.languages[0].clone(),
                    path: vec![],
                    dir: vec![],
                    document: self.1.clone(),
                    order: 0,
                    origin: "mem".into(),
                    source_rev: None,
                }],
                problems: vec![],
            }
        }
        fn media(&self) -> Listing<FileEntry> {
            Listing::default()
        }
        fn passthrough(&self) -> Listing<FileEntry> {
            Listing {
                items: vec![FileEntry {
                    area: Area::Pass,
                    path: PassPath::parse("a.css").unwrap(),
                    declared_sha256: Some([7; 32]),
                }],
                problems: vec![],
            }
        }
        fn open(&self, _: &FileEntry) -> Result<Box<dyn std::io::Read>, PassError> {
            Ok(Box::new(std::io::Cursor::new(b"a{}".to_vec())))
        }
    }
    let t = scratch(false);
    let cfg = SiteConfig::parse(&fs::read_to_string(t.path().join("site.toml")).unwrap()).unwrap();
    let s = Lying(cfg, fs_text::import_markdown("# Hi\n"));
    let e = build(&s, &t.path().join("_o"), &opts())
        .unwrap_err()
        .to_string();
    assert!(e.contains("checksum"), "{e}");
}

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
