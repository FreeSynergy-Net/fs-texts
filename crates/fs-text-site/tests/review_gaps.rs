//! Claims that had no test: the longest folder rule wins, the hash check of media, unreadable
//! folders, links between sites of one domain, the anchors of the page frame, the sitemap.
mod common;
use common::*;
use fs_text_html::pass::PassError;
use fs_text_site::{
    build, build_all, FileEntry, FolderSource, Listing, PageEntry, SiteConfig, SiteSource,
};
use std::cell::Cell;
use std::fs;
use std::io::{Cursor, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nbytes of a picture";

fn csp_of(out: &Path, page: &str) -> String {
    let h = fs::read_to_string(out.join(page)).unwrap();
    let i = h.find("Content-Security-Policy\" content=\"").unwrap() + 34;
    h[i..i + h[i..].find('"').unwrap()].to_string()
}

// ---- the longest folder rule wins -------------------------------------------------------

#[test]
fn the_longest_folder_with_a_policy_wins() {
    let strict = "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'";
    let outer = format!("{strict}; img-src 'none'");
    let inner = format!("{strict}; font-src 'none'");
    let t = scratch(false);
    let mut s = fs::read_to_string(t.path().join("site.toml")).unwrap();
    s.push_str(&format!(
        "[[csp.area]]\npath = \"/apps/x\"\npolicy = \"{inner}\"\n[[csp.area]]\npath = \"/apps\"\npolicy = \"{outer}\"\n"
    ));
    fs::write(t.path().join("site.toml"), s).unwrap();
    put(&t, "content/en/apps/a.md", b"# A\n");
    put(&t, "content/en/apps/x/p.md", b"# P\n");
    put(&t, "content/en/apps/x/deep/q.md", b"# Q\n");
    let out = t.path().join("_o");
    build(&FolderSource::open(t.path()).unwrap(), &out, &opts()).expect("builds");
    let (o, i) = (outer.replace('\'', "&#39;"), inner.replace('\'', "&#39;"));
    assert_eq!(csp_of(&out, "en/apps/a/index.html"), o);
    assert_eq!(csp_of(&out, "en/apps/x/p/index.html"), i, "the longer one");
    assert_eq!(
        csp_of(&out, "en/apps/x/deep/q/index.html"),
        i,
        "and below it"
    );
}

// ---- a media file that changes between check and copy ----------------------------------

struct FlakyMedia {
    inner: FolderSource,
    opens: Cell<usize>,
    flip: bool,
}

impl SiteSource for FlakyMedia {
    fn config(&self) -> &SiteConfig {
        self.inner.config()
    }
    fn pages(&self) -> Listing<PageEntry> {
        self.inner.pages()
    }
    fn media(&self) -> Listing<FileEntry> {
        self.inner.media()
    }
    fn passthrough(&self) -> Listing<FileEntry> {
        self.inner.passthrough()
    }
    fn open(&self, f: &FileEntry) -> Result<Box<dyn Read>, PassError> {
        if self.flip && f.path.as_str().ends_with("pic.png") {
            let n = self.opens.get();
            self.opens.set(n + 1);
            if n >= 1 {
                let mut b = PNG.to_vec();
                b.push(b'!');
                return Ok(Box::new(Cursor::new(b)));
            }
        }
        self.inner.open(f)
    }
}

#[test]
fn a_media_file_that_changes_between_check_and_copy_stops_the_build() {
    let t = scratch(false);
    put(&t, "content/en/pic.png", PNG);
    put(
        &t,
        "content/en/index.md",
        b"# Hi\n\n![a picture](pic.png)\n",
    );
    let flaky = |flip| FlakyMedia {
        inner: FolderSource::open(t.path()).unwrap(),
        opens: Cell::new(0),
        flip,
    };
    let out = t.path().join("_o");
    let e = build(&flaky(true), &out, &opts()).expect_err("must stop");
    assert!(e.to_string().contains("changed while the build ran"), "{e}");
    assert!(!out.exists(), "nothing may be left behind");
    // counter-probe: the same source that stays the same builds
    let out = t.path().join("_o2");
    build(&flaky(false), &out, &opts()).expect("builds");
}

// ---- unreadable folders -----------------------------------------------------------------

#[test]
fn an_unreadable_folder_is_reported_and_stops_the_build() {
    for (rel, kind) in [("content/en/sub", "content"), ("files/sub", "files")] {
        let t = scratch(false);
        put(&t, &format!("{rel}/a.md"), b"# A\n");
        let dir = t.path().join(rel);
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read_dir(&dir).is_ok() {
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
            eprintln!("skipped: running as root");
            return;
        }
        let e = refused(&t);
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(
            e.contains("cannot be read") && e.contains("sub"),
            "{kind}: {e}"
        );
    }
}

#[test]
fn an_entry_that_cannot_be_inspected_is_reported_not_skipped() {
    let t = scratch(false);
    put(&t, "content/en/sub/a.md", b"# A\n");
    let dir = t.path().join("content/en/sub");
    // readable (names are listed) but not searchable: every `stat` of an entry fails
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o400)).unwrap();
    if fs::symlink_metadata(dir.join("a.md")).is_ok() {
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        eprintln!("skipped: running as root");
        return;
    }
    let e = refused(&t);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(e.contains("cannot be inspected"), "{e}");
}

// ---- links between the sites of one domain ----------------------------------------------

fn two_sites(wiki_text: &str) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let site = |dir: &str, id: &str, prefix: &str, langs: &str, text: &str| {
        let toml = format!("schema = 2\npurpose = \"website\"\n[site]\nid = \"{id}\"\npath_prefix = \"{prefix}\"\nlanguages = {langs}\n[names]\nprimary = \"example.test\"\n[acme]\nmethod = \"http-01\"\n");
        put_at(&t.path().join(dir).join("site.toml"), toml.as_bytes());
        let lang = if langs.contains("de") { "de" } else { "en" };
        put_at(
            &t.path().join(dir).join(format!("content/{lang}/index.md")),
            text.as_bytes(),
        );
    };
    site(
        "landing",
        "landing",
        "/",
        "[\"de\"]",
        "# Start\n\n[to the wiki](/wiki/en/)\n",
    );
    site("wiki", "wiki", "/wiki", "[\"en\"]", wiki_text);
    t
}

#[test]
fn a_link_from_the_wiki_to_the_landing_page_is_not_dead() {
    let t = two_sites("# Wiki\n\n[home](/de/)\n\n[sitemap](/sitemap.xml)\n");
    let out = t.path().join("out");
    let r = build_all(t.path(), &out, &opts()).expect("builds: the target site exists");
    assert!(r.iter().all(|(_, r)| r.dead_links == 0));
    let h = fs::read_to_string(out.join("wiki/en/index.html")).unwrap();
    assert!(
        h.contains("href=\"/de/\"") && h.contains("href=\"/sitemap.xml\""),
        "{h}"
    );
    // the other direction was found before: landing -> wiki
    let h = fs::read_to_string(out.join("landing/de/index.html")).unwrap();
    assert!(h.contains("href=\"/wiki/en/\""), "{h}");
}

#[test]
fn a_link_to_a_place_no_site_owns_is_still_dead_and_so_is_one_without_the_set() {
    let t = two_sites("# Wiki\n\n[nowhere](/nirgendwo/)\n");
    let e = build_all(t.path(), &t.path().join("out"), &opts()).unwrap_err();
    assert!(e.to_string().contains("/nirgendwo/"), "{e}");
    // built alone, the wiki does not know the landing site: its link is dead
    let t = two_sites("# Wiki\n\n[home](/de/)\n");
    let src = FolderSource::open(t.path().join("wiki")).unwrap();
    let e = build(&src, &t.path().join("alone"), &opts()).unwrap_err();
    assert!(e.to_string().contains("/de/"), "{e}");
}

#[test]
fn the_package_files_of_wiki_and_landing_are_schema_2_with_two_ids() {
    let t = two_sites("# Wiki\n");
    let out = t.path().join("out");
    build_all(t.path(), &out, &opts()).unwrap();
    let read = |d: &str| {
        let s = fs::read_to_string(out.join(d).join("site.toml")).unwrap();
        assert!(s.starts_with("schema = 2\n") && !s.contains("host"), "{s}");
        SiteConfig::parse(&s).unwrap_or_else(|e| panic!("{d}: {e}\n{s}"))
    };
    let (landing, wiki) = (read("landing"), read("wiki"));
    assert_ne!(landing.id, wiki.id);
    assert_eq!(
        (landing.url_prefix().as_str(), wiki.url_prefix().as_str()),
        ("", "/wiki")
    );
    // together they pass the checks across sites again (what the render step does)
    fs_text_site::SiteSet::new(vec![landing, wiki]).expect("no collision");
}

// ---- the anchors of the page frame ------------------------------------------------------

#[test]
fn a_link_to_the_main_anchor_of_the_frame_is_not_dead() {
    let t = scratch(false);
    put(
        &t,
        "content/en/index.md",
        b"# Hi\n\n[skip](#main) and [other](other/#main)\n",
    );
    put(&t, "content/en/other.md", b"# Other\n");
    let out = t.path().join("_o");
    let r = build(&FolderSource::open(t.path()).unwrap(), &out, &opts()).expect("builds");
    assert_eq!(r.dead_links, 0, "{}", r.to_text());
    // counter-probe: a name nobody has is still dead
    put(&t, "content/en/index.md", b"# Hi\n\n[x](#mainx)\n");
    assert!(refused(&t).contains("mainx"));
}

// ---- the sitemap lists the contents pages -----------------------------------------------

#[test]
fn the_sitemap_lists_the_contents_page_of_every_language_with_its_variants() {
    let t = two_sites("# Wiki\n");
    let out = t.path().join("out");
    build_all(t.path(), &out, &opts()).unwrap();
    let s = fs::read_to_string(out.join("landing/sitemap.xml")).unwrap();
    assert!(
        s.contains("<loc>https://example.test/de/inhalt/</loc>"),
        "{s}"
    );
    let s = fs::read_to_string(out.join("wiki/sitemap.xml")).unwrap();
    assert!(
        s.contains("<loc>https://example.test/wiki/en/contents/</loc>"),
        "{s}"
    );
    assert!(s.contains("hreflang=\"x-default\""));
}
