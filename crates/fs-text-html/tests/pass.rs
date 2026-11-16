//! Pass-through: path rules, media rules, byte equality, and the file system cases.
use fs_text_html::pass::{MediaType, PassError, PassPath, PassSet, Prober};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

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
        assert!(
            format!("{e:?}").starts_with(kind),
            "{p:?}: {e:?}, wanted {kind}"
        );
    }
    assert!(PassPath::parse(&"a/".repeat(13)).is_err());
    assert!(PassPath::parse(&"a".repeat(101)).is_err());
    assert!(PassPath::parse(&format!("{}.txt", "a/".repeat(100))).is_err());
}

#[test]
fn good_paths_are_accepted() {
    for p in [
        "index.html",
        "apps/demo/app.v2.js",
        ".well-known/security.txt",
        "a_b-c/D.PNG",
    ] {
        assert!(PassPath::parse(p).is_ok(), "{p}");
    }
    assert_eq!(
        PassPath::parse("a/B.PNG").unwrap().extension().as_deref(),
        Some("png")
    );
    assert_eq!(
        PassPath::parse(".hidden").err().map(|e| e.to_string()),
        Some("hidden file or folder".into())
    );
}

#[test]
fn collisions_are_refused() {
    let p = |s| PassPath::parse(s).unwrap();
    let mut set = PassSet::new(&["de", "en", "_fs", "sitemap.xml"]);
    set.add(&p("apps/x.html")).unwrap();
    assert!(matches!(
        set.add(&p("apps/x.html")),
        Err(PassError::Duplicate(_))
    ));
    assert!(
        matches!(set.add(&p("Apps/X.html")), Err(PassError::Duplicate(_))),
        "only the case differs"
    );
    assert!(
        matches!(
            set.add(&p("apps/x.html/y.js")),
            Err(PassError::FileAndFolder(_))
        ),
        "file then folder"
    );
    assert!(
        matches!(set.add(&p("apps")), Err(PassError::FileAndFolder(_))),
        "folder then file"
    );
    for r in [
        "de/a.html",
        "DE/a.html",
        "en/x.css",
        "_fs/theme.css",
        "sitemap.xml",
    ] {
        assert!(matches!(set.add(&p(r)), Err(PassError::Reserved(_))), "{r}");
    }
    assert!(
        set.add(&p("deep/x.html")).is_ok(),
        "a name that only starts like a reserved one"
    );
}

#[test]
fn media_types_and_unknown_extensions() {
    assert_eq!(
        MediaType::from_extension("html").unwrap().mime(),
        "text/html"
    );
    assert_eq!(MediaType::from_extension("jpeg").unwrap(), MediaType::Jpeg);
    assert_eq!(
        MediaType::from_extension("webmanifest").unwrap().mime(),
        "application/manifest+json"
    );
    for e in ["exe", "wasm", "php", "", "htm", "sh", "bin"] {
        assert!(
            matches!(
                MediaType::from_extension(e),
                Err(PassError::UnknownExtension(_))
            ),
            "{e}"
        );
    }
    for (e, a) in [
        ("html", true),
        ("js", true),
        ("mjs", true),
        ("svg", true),
        ("css", false),
        ("png", false),
        ("json", false),
    ] {
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
    assert!(
        probe("png", b"<html>not a png</html>").is_err(),
        ".png with HTML inside"
    );
    assert!(probe("png", b"").is_err());
    assert!(probe("jpg", b"\xff\xd8\xff\xe0abc").is_ok() && probe("jpg", PNG).is_err());
    assert!(probe("gif", b"GIF89a....").is_ok() && probe("webp", b"RIFF\0\0\0\0WEBPVP8 ").is_ok());
    assert!(probe("avif", b"\0\0\0\x1cftypavif....").is_ok());
    assert!(probe("pdf", b"%PDF-1.7\n").is_ok() && probe("pdf", b"<html>").is_err());
    assert!(probe("woff2", b"wOF2....").is_ok() && probe("ico", &[0, 0, 1, 0, 1]).is_ok());
    assert!(
        probe("html", "<p>\u{e4}\u{20ac}\u{1f600}</p>".as_bytes()).is_ok(),
        "multi-byte characters across chunks"
    );
    assert!(probe("html", PNG).is_err(), ".html with PNG inside");
    assert!(probe("txt", b"a\0b").is_err());
    assert!(probe("css", b"a{}\xff").is_err());
    assert!(
        probe("js", &"\u{20ac}".as_bytes()[..2]).is_err(),
        "cut-off character at the end"
    );
    assert!(
        probe("svg", b"<?xml?><svg xmlns=''/>").is_ok() && probe("svg", b"<html></html>").is_err()
    );
    assert!(probe("xml", b"<a/>").is_ok() && probe("xml", b"plain").is_err());
}

#[test]
fn reserved_paths_are_built_in_and_may_have_several_segments() {
    use fs_text_html::pass::{PassPath, PassSet};
    let mut set = PassSet::new(&["apps/private"]);
    for p in [
        "site.toml",
        "SITE.TOML",
        "site-manifest.json",
        "_fs/media/x.png",
        "sitemap.xml",
        ".well-known/acme-challenge/token.txt",
        "apps/private/x.css",
        "apps/private",
    ] {
        assert!(set.add(&PassPath::parse(p).unwrap()).is_err(), "{p}");
    }
    for p in [
        ".well-known/security.txt",
        "apps/public/x.css",
        "apps/privatex.css",
        "site.toml.txt",
    ] {
        assert!(set.add(&PassPath::parse(p).unwrap()).is_ok(), "{p}");
    }
}
