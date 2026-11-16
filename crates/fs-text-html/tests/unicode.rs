//! Non-ASCII text behind tags, comments and entities (B1 of the review of core 1: the scanner
//! cut raw HTML by bytes and panicked on `<div>ä</div>`), also in the exporter and as proptest.
mod common;
use common::{html, page};
use fs_text::{import_markdown, Exporter};
use fs_text_html::HtmlExporter;
use proptest::prelude::*;

const SAMPLES: &[&str] = &[
    "<div>ä</div>",
    "<!-- x -->ä",
    "&amp;ä",
    "<summary>Übersicht</summary>",
    "<kbd>ß</kbd>ß",
    "a<sub>日本語</sub>b",
    "<details><summary>Ärger 😀</summary>Text 😀</details>",
    "&;ä &#x1F600;😀 &ouml;ö",
    "ä<",
    "<ä>",
    "<a href=\"ü\">ö</a>",
    "😀<!--",
    "x &",
    "漢字<br>漢字<u>字</u>",
    "<mark title=\"Grüße 😀\">ü</mark>",
];

#[test]
fn raw_html_with_non_ascii_does_not_panic_and_keeps_the_text() {
    for s in SAMPLES {
        let h = html(s);
        // every non-ASCII character of the input is still in the output (as text)
        for c in s.chars().filter(|c| !c.is_ascii()) {
            assert!(h.contains(c), "{s:?} lost {c:?}: {h}");
        }
        let mut d = import_markdown(s);
        let _ = HtmlExporter::default().export(&d);
        d.blocks.clear();
    }
}

#[test]
fn exact_output_for_the_cases_of_the_review() {
    assert_eq!(html("a<kbd>ä</kbd>b").trim(), "<p>a<kbd>ä</kbd>b</p>");
    assert_eq!(html("a<!-- x -->ä").trim(), "<p>aä</p>");
    assert_eq!(html("a &amp;ä").trim(), "<p>a &amp;ä</p>");
    assert!(html("<div>ä</div>").contains("&lt;div&gt;ä&lt;/div&gt;"));
}

#[test]
fn the_real_corpus_never_panics_also_with_raw_html() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fs-text/tests/corpus");
    let mut n = 0;
    for e in std::fs::read_dir(dir).expect("corpus") {
        let md = std::fs::read_to_string(e.expect("entry").path()).expect("utf-8");
        let p = page(&md);
        assert!(!p.body.contains("<script"), "{}", p.body);
        n += 1;
    }
    assert!(n > 5, "corpus not found");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]
    #[test]
    fn any_unicode_text_never_panics_and_stays_safe(s in "(\\PC|[<>&!/=\" -]){0,200}") {
        let h = page(&s);
        prop_assert!(!h.body.contains("<script"));
    }

    #[test]
    fn unicode_between_hostile_pieces(parts in prop::collection::vec(
        prop::sample::select(vec![
            "ä", "ß", "😀", "漢", "é", "<", ">", "&", "&amp;", "<!--", "-->", "<kbd>", "</kbd>", "<details>", "<summary>", "</summary>",
            "<script>", "\n\n", "\n", "`", "**", "[a](", ")", "\u{200b}", "\u{202e}", "\u{feff}",
        ]), 0..50)) {
        let h = page(&parts.concat());
        prop_assert!(!h.body.contains("<script"));
        let _ = HtmlExporter::default().export(&import_markdown(&parts.concat()));
    }
}
