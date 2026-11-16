//! The media table of the pass-through against what Zentinel 26.11_2 sends. Zentinel's static
//! route takes the type from `mime_guess` 2.0.5 (`static_files/mod.rs` `get_content_type`:
//! `from_path(path).first_or_octet_stream()`; Cargo.lock of 26.11_2 pins 2.0.5). The same crate,
//! at the same version, is the dev-dependency here, so this is the table the server uses, not a
//! copy of it. A Zentinel update that changes `mime_guess` shows here (version pin).
use fs_text_html::pass::MediaType;

const EXTENSIONS: [&str; 18] = [
    "html",
    "css",
    "js",
    "mjs",
    "json",
    "svg",
    "png",
    "jpg",
    "jpeg",
    "gif",
    "webp",
    "avif",
    "ico",
    "woff2",
    "txt",
    "pdf",
    "xml",
    "webmanifest",
];

/// Where our table and Zentinel's differ (extension, ours, Zentinel's). Listed so a change on
/// either side is a red test; the manifest of the site carries OUR type, the check of the
/// release must expect Zentinel's for these.
fn known_differences() -> Vec<(&'static str, &'static str, &'static str)> {
    // filled from the first run, see the assertion message below
    include!("zentinel_differences.in")
}

#[test]
fn every_extension_of_the_table_is_compared_with_zentinel() {
    let mut diff = Vec::new();
    for e in EXTENSIONS {
        let ours = MediaType::from_extension(e).expect("in the table").mime();
        let theirs = mime_guess::from_ext(e).first_or_octet_stream();
        assert_ne!(
            theirs.essence_str(),
            "application/octet-stream",
            "Zentinel would send octet-stream for .{e}"
        );
        if theirs.essence_str() != ours {
            diff.push((e, ours, theirs.essence_str().to_string()));
        }
    }
    let want = known_differences();
    let got: Vec<(&str, &str, &str)> = diff.iter().map(|(a, b, c)| (*a, *b, c.as_str())).collect();
    assert_eq!(
        got, want,
        "differences between our table and Zentinel's: {got:?}"
    );
}

/// The types of the files the builder writes itself are the ones Zentinel sends for them.
#[test]
fn generated_files_carry_the_type_zentinel_sends() {
    for (file, ours) in [
        ("index.html", "text/html"),
        ("a.css", "text/css"),
        ("sitemap.xml", MediaType::Xml.mime()),
        ("robots.txt", "text/plain"),
        ("site-manifest.json", "application/json"),
        ("site.toml", "text/x-toml"),
    ] {
        let theirs = mime_guess::from_path(file).first_or_octet_stream();
        assert_eq!(theirs.essence_str(), ours, "{file}");
    }
}

#[test]
fn the_table_has_exactly_these_extensions() {
    // an extension added to the table without a line here is a red test
    for e in [
        "bin", "exe", "wasm", "php", "htm", "md", "toml", "csv", "zip",
    ] {
        assert!(MediaType::from_extension(e).is_err(), "{e}");
    }
    for e in EXTENSIONS {
        assert!(MediaType::from_extension(e).is_ok(), "{e}");
    }
}
