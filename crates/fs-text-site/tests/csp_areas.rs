//! `[[csp.area]].policy`: a folder's own policy in the generated pages of that folder, never
//! looser than the site's unless `allow_loosen = true` (then the build warns).
mod common;
use common::*;
use fs_text_site::{build, FolderSource, SiteConfig};
use std::fs;

const STRICT_AREA: &str = "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; img-src 'none'";
const LOOSE_AREA: &str = "default-src 'self'; script-src 'self'";

fn with_area(policy: &str, extra: &str) -> tempfile::TempDir {
    let t = scratch(false);
    let mut s = fs::read_to_string(t.path().join("site.toml")).unwrap();
    s.push_str(&format!(
        "[[csp.area]]\npath = \"/apps\"\npolicy = \"{policy}\"\n{extra}\n"
    ));
    fs::write(t.path().join("site.toml"), s).unwrap();
    put(&t, "content/en/apps/a.md", b"# App\n");
    put(&t, "content/en/apps/deep/b.md", b"# Deep\n");
    put(&t, "content/en/other.md", b"# Other\n");
    t
}

fn csp_of(out: &std::path::Path, page: &str) -> String {
    let h = fs::read_to_string(out.join(page)).unwrap();
    let i = h.find("Content-Security-Policy\" content=\"").unwrap() + 34;
    h[i..i + h[i..].find('"').unwrap()].to_string()
}

fn built(t: &tempfile::TempDir) -> (std::path::PathBuf, fs_text_site::BuildReport) {
    let out = t.path().join("_o");
    let r = build(&FolderSource::open(t.path()).unwrap(), &out, &opts()).expect("builds");
    (out, r)
}

#[test]
fn a_stricter_folder_policy_is_written_into_the_pages_of_that_folder_only() {
    let t = with_area(STRICT_AREA, "");
    let (out, r) = built(&t);
    let want = STRICT_AREA.replace('\'', "&#39;");
    assert_eq!(csp_of(&out, "en/apps/a/index.html"), want);
    assert_eq!(
        csp_of(&out, "en/apps/deep/b/index.html"),
        want,
        "below it too"
    );
    assert_eq!(
        csp_of(&out, "en/apps/index.html"),
        want,
        "the generated overview of the folder"
    );
    assert_ne!(csp_of(&out, "en/other/index.html"), want);
    assert_ne!(csp_of(&out, "en/index.html"), want);
    assert_ne!(csp_of(&out, "en/contents/index.html"), want);
    assert!(r.findings.is_empty(), "{}", r.to_text());
}

#[test]
fn a_looser_folder_policy_is_refused() {
    for p in [
        LOOSE_AREA,
        "default-src *",
        "default-src 'self' https://cdn.test",
    ] {
        let t = with_area(p, "");
        let e = FolderSource::open(t.path()).unwrap_err().to_string();
        assert!(e.contains("allows more"), "{p}: {e}");
    }
}

#[test]
fn allow_loosen_lets_it_through_with_a_warning_and_the_policy_is_written() {
    let t = with_area(LOOSE_AREA, "allow_loosen = true");
    let (out, r) = built(&t);
    assert_eq!(
        csp_of(&out, "en/apps/a/index.html"),
        LOOSE_AREA.replace('\'', "&#39;")
    );
    let w: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.message.contains("allow_loosen"))
        .collect();
    assert_eq!(w.len(), 1, "{}", r.to_text());
    assert!(!w[0].fatal);
    // counter-probe: a strict policy with the flag raises no warning
    let t = with_area(STRICT_AREA, "allow_loosen = true");
    assert!(built(&t).1.findings.is_empty());
}

#[test]
fn the_comparison_is_made_against_a_custom_site_policy_too() {
    let site = "default-src 'self'; img-src 'self' data:";
    let t = scratch(false);
    let toml = fs::read_to_string(t.path().join("site.toml")).unwrap();
    let toml = toml.replace(
        "mode = \"passthrough\"",
        &format!("mode = \"passthrough\"\npolicy = \"{site}\""),
    );
    let ok = format!(
        "{toml}[[csp.area]]\npath = \"/a\"\npolicy = \"default-src 'self'; img-src 'self'\"\n"
    );
    assert!(SiteConfig::parse(&ok).is_ok());
    let bad = format!("{toml}[[csp.area]]\npath = \"/a\"\npolicy = \"default-src 'self'; img-src 'self' https://x.test\"\n");
    assert!(SiteConfig::parse(&bad).is_err());
}
