//! Pass-through: path rules, media rules, byte equality, and the file system cases.
use fs_text_html::pass::{copy_file, Limits, MediaType, PassError, PassPath, PassSet, Prober};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

fn setup() -> (tempfile::TempDir, tempfile::TempDir) {
    (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap())
}

fn copy(root: &Path, dest: &Path, rel: &str) -> Result<fs_text_html::pass::Copied, PassError> {
    copy_file(root, &PassPath::parse(rel)?, dest, Limits::default())
}

#[test]
fn bad_paths_are_refused() {
    let cases: &[(&str, &str)] = &[
        ("", "EmptyPath"),
        ("/etc/passwd", "Absolute"),
        ("../x.html", "DotSegment"),
        ("a/../x.html", "DotSegment"),
        ("./x.html", "DotSegment"),
        ("a//b.html", "EmptySegment"),
        ("a/", "EmptySegment"),
        ("a\\b.html", "BadChar"),
        ("a b.html", "BadChar"),
        ("\u{e4}.html", "BadChar"),
        ("a\0b.html", "BadChar"),
        ("a\nb.html", "BadChar"),
        (".git/config", "Hidden"),
        ("a/.env", "Hidden"),
        (".well-known/../x", "DotSegment"),
        ("a/.well-known/x.txt", "Hidden"),
    ];
    for (p, kind) in cases {
        let e = PassPath::parse(p).expect_err(p);
        assert!(format!("{e:?}").starts_with(kind), "{p:?}: {e:?}, wanted {kind}");
    }
    assert!(PassPath::parse(&"a/".repeat(13)).is_err());
    assert!(PassPath::parse(&"a".repeat(101)).is_err());
    assert!(PassPath::parse(&format!("{}.txt", "a/".repeat(100))).is_err());
}

#[test]
fn good_paths_are_accepted() {
    for p in ["index.html", "apps/demo/app.v2.js", ".well-known/security.txt", "a_b-c/D.PNG"] {
        assert!(PassPath::parse(p).is_ok(), "{p}");
    }
    assert_eq!(PassPath::parse("a/B.PNG").unwrap().extension().as_deref(), Some("png"));
    assert_eq!(PassPath::parse(".hidden").err().map(|e| e.to_string()), Some("hidden file or folder".into()));
}

#[test]
fn collisions_are_refused() {
    let p = |s| PassPath::parse(s).unwrap();
    let mut set = PassSet::new(&["de", "en", "_fs", "sitemap.xml"]);
    set.add(&p("apps/x.html")).unwrap();
    assert!(matches!(set.add(&p("apps/x.html")), Err(PassError::Duplicate(_))));
    assert!(matches!(set.add(&p("Apps/X.html")), Err(PassError::Duplicate(_))), "only the case differs");
    assert!(matches!(set.add(&p("apps/x.html/y.js")), Err(PassError::FileAndFolder(_))), "file then folder");
    assert!(matches!(set.add(&p("apps")), Err(PassError::FileAndFolder(_))), "folder then file");
    for r in ["de/a.html", "DE/a.html", "en/x.css", "_fs/theme.css", "sitemap.xml"] {
        assert!(matches!(set.add(&p(r)), Err(PassError::Reserved(_))), "{r}");
    }
    assert!(set.add(&p("deep/x.html")).is_ok(), "a name that only starts like a reserved one");
}

#[test]
fn media_types_and_unknown_extensions() {
    assert_eq!(MediaType::from_extension("html").unwrap().mime(), "text/html");
    assert_eq!(MediaType::from_extension("jpeg").unwrap(), MediaType::Jpeg);
    assert_eq!(MediaType::from_extension("webmanifest").unwrap().mime(), "application/manifest+json");
    for e in ["exe", "wasm", "php", "", "htm", "sh", "bin"] {
        assert!(matches!(MediaType::from_extension(e), Err(PassError::UnknownExtension(_))), "{e}");
    }
    for (e, a) in [("html", true), ("js", true), ("mjs", true), ("svg", true), ("css", false), ("png", false), ("json", false)] {
        assert_eq!(MediaType::from_extension(e).unwrap().is_active(), a, "{e}");
    }
}

fn probe(ext: &str, bytes: &[u8]) -> Result<(), PassError> {
    let mut p = Prober::new(MediaType::from_extension(ext).unwrap());
    for c in bytes.chunks(3) {
        p.feed(c)?;
    }
    p.finish()
}

#[test]
fn content_must_fit_the_extension() {
    assert!(probe("png", PNG).is_ok());
    assert!(probe("png", b"<html>not a png</html>").is_err(), ".png with HTML inside");
    assert!(probe("png", b"").is_err());
    assert!(probe("jpg", b"\xff\xd8\xff\xe0abc").is_ok() && probe("jpg", PNG).is_err());
    assert!(probe("gif", b"GIF89a....").is_ok() && probe("webp", b"RIFF\0\0\0\0WEBPVP8 ").is_ok());
    assert!(probe("avif", b"\0\0\0\x1cftypavif....").is_ok());
    assert!(probe("pdf", b"%PDF-1.7\n").is_ok() && probe("pdf", b"<html>").is_err());
    assert!(probe("woff2", b"wOF2....").is_ok() && probe("ico", &[0, 0, 1, 0, 1]).is_ok());
    assert!(probe("html", "<p>\u{e4}\u{20ac}\u{1f600}</p>".as_bytes()).is_ok(), "multi-byte characters across chunks");
    assert!(probe("html", PNG).is_err(), ".html with PNG inside");
    assert!(probe("txt", b"a\0b").is_err());
    assert!(probe("css", b"a{}\xff").is_err());
    assert!(probe("js", &"\u{20ac}".as_bytes()[..2]).is_err(), "cut-off character at the end");
    assert!(probe("svg", b"<?xml?><svg xmlns=''/>").is_ok() && probe("svg", b"<html></html>").is_err());
    assert!(probe("xml", b"<a/>").is_ok() && probe("xml", b"plain").is_err());
}

#[test]
fn copy_is_byte_for_byte_and_hashes_match() {
    let (root, dest) = setup();
    let page = b"<!doctype html><script>alert('mine')</script><p>\xc3\xa4</p>\r\n\t  trailing   ".to_vec();
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("index.html", page),
        ("apps/demo/app.js", b"let x=1;\n// no newline at end".to_vec()),
        ("apps/demo/style.css", b"a{color:red}".to_vec()),
        ("apps/demo/logo.svg", b"<svg onload=\"x()\"></svg>".to_vec()),
        ("img/p.png", [PNG, &(0..=255u8).cycle().take(200_000).collect::<Vec<_>>()].concat()),
        (".well-known/security.txt", b"Contact: x\n".to_vec()),
    ];
    for (rel, bytes) in &files {
        let p = root.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
        let c = copy(root.path(), dest.path(), rel).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let out = fs::read(dest.path().join(rel)).unwrap();
        assert_eq!(&out, bytes, "{rel}: the bytes changed");
        let want: [u8; 32] = Sha256::digest(bytes).into();
        assert_eq!((c.sha256, c.bytes), (want, bytes.len() as u64), "{rel}");
        assert_eq!(Sha256::digest(&out).as_slice(), want);
    }
}

#[test]
fn nothing_is_overwritten_and_a_refused_file_leaves_nothing() {
    let (root, dest) = setup();
    fs::write(root.path().join("a.html"), "<p>a</p>").unwrap();
    fs::write(root.path().join("b.png"), "<html>").unwrap();
    fs::write(dest.path().join("a.html"), "existing").unwrap();
    assert!(matches!(copy(root.path(), dest.path(), "a.html"), Err(PassError::Io(_))));
    assert_eq!(fs::read_to_string(dest.path().join("a.html")).unwrap(), "existing");
    assert!(copy(root.path(), dest.path(), "b.png").is_err());
    assert!(!dest.path().join("b.png").exists(), "a half written output is removed");
}

#[test]
fn size_limit_is_enforced() {
    let (root, dest) = setup();
    fs::write(root.path().join("big.txt"), vec![b'a'; 1000]).unwrap();
    let p = PassPath::parse("big.txt").unwrap();
    assert!(matches!(copy_file(root.path(), &p, dest.path(), Limits { max_bytes: 999 }), Err(PassError::TooBig(999))));
    assert!(copy_file(root.path(), &p, tempfile::tempdir().unwrap().path(), Limits { max_bytes: 1000 }).is_ok());
}

#[test]
fn links_in_the_source_are_refused() {
    let (root, dest) = setup();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    fs::write(root.path().join("real.txt"), "real").unwrap();
    symlink(outside.path().join("secret.txt"), root.path().join("link.txt")).unwrap();
    symlink("real.txt", root.path().join("inner.txt")).unwrap();
    symlink(outside.path(), root.path().join("dir")).unwrap();
    for rel in ["link.txt", "inner.txt", "dir/secret.txt"] {
        let e = copy(root.path(), dest.path(), rel).expect_err(rel);
        assert!(matches!(e, PassError::NotRegular(_)), "{rel}: {e:?}");
        assert!(!dest.path().join(rel).exists(), "{rel}");
    }
}

#[test]
fn hard_links_pipes_and_folders_are_refused() {
    let (root, dest) = setup();
    fs::write(root.path().join("one.txt"), "x").unwrap();
    fs::hard_link(root.path().join("one.txt"), root.path().join("two.txt")).unwrap();
    fs::create_dir(root.path().join("folder.txt")).unwrap();
    assert!(std::process::Command::new("mkfifo").arg(root.path().join("pipe.txt")).status().unwrap().success());
    for rel in ["one.txt", "two.txt", "folder.txt", "pipe.txt", "missing.txt"] {
        assert!(copy(root.path(), dest.path(), rel).is_err(), "{rel}");
    }
    assert!(fs::read_dir(dest.path()).unwrap().next().is_none(), "nothing was written");
}

#[test]
fn path_escape_cannot_reach_a_file_outside_the_folder() {
    let (root, dest) = setup();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("o.txt"), "outside").unwrap();
    for rel in ["../o.txt", "/etc/hostname", "a/../../o.txt", &outside.path().join("o.txt").to_string_lossy()] {
        assert!(copy(root.path(), dest.path(), rel).is_err(), "{rel}");
    }
}
