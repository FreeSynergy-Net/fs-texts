//! The builder on fixture sites: golden files per site, byte equality of passed files,
//! determinism, security refusals.
use fs_text_site::{build, build_all, BuildOptions, FolderSource, Gate, SiteError};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}
fn opts() -> BuildOptions {
    BuildOptions::new(UNIX_EPOCH + Duration::from_secs(1_700_000_000))
}

/// path -> (bytes, mtime secs, mode) of a whole folder.
fn snapshot(root: &Path) -> BTreeMap<String, (Vec<u8>, u64, u32)> {
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

fn check_golden(site: &str, out: &Path, files: &[&str]) {
    for f in files {
        let got = fs::read_to_string(out.join(f)).unwrap();
        let g = golden().join(site).join(f);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            fs::create_dir_all(g.parent().unwrap()).unwrap();
            fs::write(&g, &got).unwrap();
        }
        let want = fs::read_to_string(&g).unwrap_or_else(|_| panic!("missing golden {site}/{f}"));
        assert_eq!(got, want, "golden {site}/{f} (UPDATE_GOLDEN=1, then review)");
    }
}

fn build_multi(out: &Path) {
    let r = build_all(&fixtures().join("multi"), out, &opts()).expect("multi builds");
    assert_eq!(r.len(), 2);
}

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
    assert!(fs::read_to_string(out.join("landing/index.html")).unwrap().contains("Von Hand"));
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
    for rel in ["index.html", "app.js", "css/site.css", "img/logo.png", ".well-known/security.txt"] {
        let a = fs::read(src.join(rel)).unwrap();
        let b = fs::read(out.join("landing").join(rel)).unwrap();
        assert_eq!(a, b, "{rel}");
        let e = manifest["files"].as_array().unwrap().iter().find(|e| e["path"] == rel).unwrap();
        assert_eq!(e["sha256"], format!("{:x}", Sha256::digest(&a)));
        assert_eq!(e["kind"], "passthrough");
        assert_eq!(e["active"], rel.ends_with(".html") || rel.ends_with(".js"));
        assert_eq!(e["bytes"], a.len());
    }
    // every file in the folder (but the manifest) is in the manifest, with the right hash
    let snap = snapshot(&out.join("landing"));
    let listed: Vec<&str> = manifest["files"].as_array().unwrap().iter().map(|e| e["path"].as_str().unwrap()).collect();
    for (p, (bytes, _, _)) in &snap {
        if p.ends_with('/') || p == "site-manifest.json" {
            continue;
        }
        assert!(listed.contains(&p.as_str()), "{p} missing in manifest");
        let e = manifest["files"].as_array().unwrap().iter().find(|e| e["path"] == p.as_str()).unwrap();
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
    let name = media[0].as_ref().unwrap().file_name().into_string().unwrap();
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
    check_golden("legacy", &t.path().join("o"), &["en/index.html", "site.toml"]);
}

// ---- refusals -------------------------------------------------------------------------

/// A scratch site with `site.toml` v2 (passthrough mode unless `static_mode`).
fn scratch(static_mode: bool) -> tempfile::TempDir {
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

fn put(t: &tempfile::TempDir, rel: &str, bytes: &[u8]) {
    let p = t.path().join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

fn refused(t: &tempfile::TempDir) -> String {
    let out = t.path().join("_out");
    let src = FolderSource::open(t.path()).unwrap();
    let e = build(&src, &out, &opts()).expect_err("must be refused");
    assert!(!out.exists(), "nothing may be left behind after a refusal");
    e.to_string()
}

fn builds(t: &tempfile::TempDir) -> bool {
    let src = FolderSource::open(t.path()).unwrap();
    build(&src, &t.path().join("_out2"), &opts()).is_ok()
}

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
    for p in ["en/x.html", "_fs/x.css", "sitemap.xml", "site-manifest.json", "site.toml"] {
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
    assert!(refused(&t).contains("security.txt"));
    let t = scratch(false);
    put(&t, "files/.well-known/security.txt", b"Contact: mailto:a@example.test\n");
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
    let e = build(&src, &t.path().join("_o"), &o).unwrap_err().to_string();
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
    for p in ["content/en/a.html", "content/en/a.svg", "content/en/a.js", "content/en/a.css"] {
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
    assert!(r.findings.iter().any(|f| f.message.contains("points to nothing") && !f.fatal));
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
    assert!(!html.contains("<a href=\"../") && !html.contains("javascript:") && !html.contains("%2e"));
    assert!(r.findings.len() >= 4);
}

#[test]
fn a_declared_checksum_that_does_not_match_is_refused() {
    use fs_text_site::{Area, FileEntry, Listing, PageEntry, SiteConfig, SiteSource};
    use fs_text_html::pass::{PassError, PassPath};
    struct Lying(SiteConfig, fs_text::Document);
    impl SiteSource for Lying {
        fn config(&self) -> &SiteConfig { &self.0 }
        fn pages(&self) -> Listing<PageEntry> {
            Listing { items: vec![PageEntry {
                lang: self.0.languages[0].clone(), path: vec![], dir: vec![], document: self.1.clone(),
                order: 0, origin: "mem".into(), source_rev: None }], problems: vec![] }
        }
        fn media(&self) -> Listing<FileEntry> { Listing::default() }
        fn passthrough(&self) -> Listing<FileEntry> {
            Listing { items: vec![FileEntry { area: Area::Pass, path: PassPath::parse("a.css").unwrap(), declared_sha256: Some([7; 32]) }], problems: vec![] }
        }
        fn open(&self, _: &FileEntry) -> Result<Box<dyn std::io::Read>, PassError> {
            Ok(Box::new(std::io::Cursor::new(b"a{}".to_vec())))
        }
    }
    let t = scratch(false);
    let cfg = SiteConfig::parse(&fs::read_to_string(t.path().join("site.toml")).unwrap()).unwrap();
    let s = Lying(cfg, fs_text::import_markdown("# Hi\n"));
    let e = build(&s, &t.path().join("_o"), &opts()).unwrap_err().to_string();
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
    let e = build_all(&root, &t.path().join("o"), &opts()).unwrap_err().to_string();
    assert!(e.contains("generated prefix"), "{e}");
}

fn put_at(p: &Path, b: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, b).unwrap();
}

fn copy_tree(from: &Path, to: &Path) {
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
fn colliding_sites_stop_before_anything_is_written() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("sites");
    copy_tree(&fixtures().join("multi/landing"), &root.join("one"));
    copy_tree(&fixtures().join("multi/landing"), &root.join("two"));
    let out = t.path().join("o");
    let e = build_all(&root, &out, &opts()).unwrap_err();
    assert!(matches!(e, SiteError::Config(_)));
    assert!(!out.exists());
}
