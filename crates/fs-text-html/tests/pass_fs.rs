//! Pass-through on the file system: byte equality, links, pipes, escapes, limits.
use fs_text_html::pass::{copy_file, Limits, PassError, PassPath};
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
fn copy_is_byte_for_byte_and_hashes_match() {
    let (root, dest) = setup();
    let page =
        b"<!doctype html><script>alert('mine')</script><p>\xc3\xa4</p>\r\n\t  trailing   ".to_vec();
    let files: Vec<(&str, Vec<u8>)> = vec![
        ("index.html", page),
        (
            "apps/demo/app.js",
            b"let x=1;\n// no newline at end".to_vec(),
        ),
        ("apps/demo/style.css", b"a{color:red}".to_vec()),
        ("apps/demo/logo.svg", b"<svg onload=\"x()\"></svg>".to_vec()),
        (
            "img/p.png",
            [PNG, &(0..=255u8).cycle().take(200_000).collect::<Vec<_>>()].concat(),
        ),
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
    assert!(matches!(
        copy(root.path(), dest.path(), "a.html"),
        Err(PassError::Io(_))
    ));
    assert_eq!(
        fs::read_to_string(dest.path().join("a.html")).unwrap(),
        "existing"
    );
    assert!(copy(root.path(), dest.path(), "b.png").is_err());
    assert!(
        !dest.path().join("b.png").exists(),
        "a half written output is removed"
    );
}

#[test]
fn size_limit_is_enforced() {
    let (root, dest) = setup();
    fs::write(root.path().join("big.txt"), vec![b'a'; 1000]).unwrap();
    let p = PassPath::parse("big.txt").unwrap();
    assert!(matches!(
        copy_file(root.path(), &p, dest.path(), Limits { max_bytes: 999 }),
        Err(PassError::TooBig(999))
    ));
    assert!(copy_file(
        root.path(),
        &p,
        tempfile::tempdir().unwrap().path(),
        Limits { max_bytes: 1000 }
    )
    .is_ok());
}

#[test]
fn links_in_the_source_are_refused() {
    let (root, dest) = setup();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    fs::write(root.path().join("real.txt"), "real").unwrap();
    symlink(
        outside.path().join("secret.txt"),
        root.path().join("link.txt"),
    )
    .unwrap();
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
    assert!(std::process::Command::new("mkfifo")
        .arg(root.path().join("pipe.txt"))
        .status()
        .unwrap()
        .success());
    for rel in [
        "one.txt",
        "two.txt",
        "folder.txt",
        "pipe.txt",
        "missing.txt",
    ] {
        assert!(copy(root.path(), dest.path(), rel).is_err(), "{rel}");
    }
    assert!(
        fs::read_dir(dest.path()).unwrap().next().is_none(),
        "nothing was written"
    );
}

#[test]
fn path_escape_cannot_reach_a_file_outside_the_folder() {
    let (root, dest) = setup();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("o.txt"), "outside").unwrap();
    for rel in [
        "../o.txt",
        "/etc/hostname",
        "a/../../o.txt",
        &outside.path().join("o.txt").to_string_lossy(),
    ] {
        assert!(copy(root.path(), dest.path(), rel).is_err(), "{rel}");
    }
}
