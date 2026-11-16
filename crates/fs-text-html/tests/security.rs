//! Raw HTML, link schemes and injection: what must never reach the output.
mod common;
use common::{html, page};
use fs_text::{import_markdown, Document, Exporter};
use fs_text_html::url::{is_relative, safe_link};
use fs_text_html::{HtmlExporterBuilder, Problem, RawHtmlPolicy};
use proptest::prelude::*;

#[test]
fn unsafe_schemes_are_refused() {
    for bad in [
        "javascript:alert(1)",
        "JaVaScRiPt:alert(1)",
        " javascript:x",
        "java\tscript:x",
        "java\nscript:x",
        "data:text/html,x",
        "vbscript:x",
        "file:///etc/passwd",
        "//evil.example/x",
        "/\\evil.example",
        "ftp://a/b",
        "",
    ] {
        assert!(!safe_link(bad), "{bad:?} must be refused");
    }
    for ok in [
        "https://a.b/c?d=1#e",
        "http://a.b",
        "mailto:a@b.c",
        "a/b.md",
        "../up",
        "#frag",
        "?q=1",
        "/abs/path",
    ] {
        assert!(safe_link(ok), "{ok:?} must be allowed");
    }
    assert!(is_relative("a/b.png") && !is_relative("https://a.b/c.png"));
}

#[test]
fn unsafe_link_becomes_text_and_is_reported() {
    let p = page("[x](javascript:alert(1)) and [y](data:text/html,z)");
    assert!(!p.body.contains("href"), "{}", p.body);
    assert!(p.body.contains("x and y"), "{}", p.body);
    assert!(p.report.problems.contains(&Problem::UnsafeLink {
        path: "javascript:alert(1)".into()
    }));
}

#[test]
fn raw_html_blocks_keep_only_the_allow_list() {
    let h = html("<div onclick=\"x()\">a</div>\n");
    assert!(!h.contains("<div onclick"), "{h}");
    assert!(
        h.contains("&lt;div onclick=&quot;x()&quot;&gt;")
            || h.contains("&lt;div onclick=\"x()\"&gt;"),
        "{h}"
    );
    let h = html("<details lang=\"de\" onclick=\"x()\" title=\"T &amp; U\">\n<summary>S</summary>\n\nbody\n\n</details>\n");
    assert!(
        h.contains("<details lang=\"de\" title=\"T &amp; U\">"),
        "{h}"
    );
    assert!(!h.contains("onclick"), "{h}");
    assert!(h.contains("<summary>S</summary>"), "{h}");
    assert_eq!(
        h.matches("<details").count(),
        h.matches("</details>").count(),
        "{h}"
    );
}

#[test]
fn raw_html_script_style_iframe_are_text() {
    for tag in [
        "<script>alert(1)</script>",
        "<iframe src=\"x\"></iframe>",
        "<style>a{}</style>",
        "<img src=x onerror=y>",
        "<svg onload=y>",
        "<object data=x>",
    ] {
        let h = html(&format!("text\n\n{tag}\n"));
        let p = page(&format!("a {tag} b"));
        for out in [&h, &p.body] {
            let l = out.to_ascii_lowercase();
            for needle in ["<script", "<iframe", "<style", "<img", "<svg", "<object"] {
                assert!(!l.contains(needle), "{tag}: {needle} in {out}");
            }
        }
        assert!(!p.report.is_clean(), "{tag}");
    }
}

#[test]
fn inline_allow_list_tags_stay_nested_inside_marks() {
    let p = page("**a <kbd>b** c</kbd> d");
    let b = &p.body;
    assert_eq!(
        b.matches("<kbd>").count(),
        b.matches("</kbd>").count(),
        "{b}"
    );
    assert!(
        b.find("</kbd>").unwrap() < b.find("</strong>").unwrap(),
        "{b}"
    );
}

#[test]
fn unclosed_allowed_tags_are_closed_and_stray_closers_become_text() {
    let b = html("a <sup>b\n");
    assert_eq!(
        b.matches("<sup>").count(),
        b.matches("</sup>").count(),
        "{b}"
    );
    let b = html("a </kbd> b\n");
    assert!(!b.contains("</kbd>") && b.contains("&lt;/kbd&gt;"), "{b}");
}

#[test]
fn escape_all_policy_writes_everything_as_text() {
    let e = HtmlExporterBuilder::default()
        .raw(RawHtmlPolicy::EscapeAll)
        .build();
    let b = e.export(&import_markdown("a <kbd>b</kbd>\n"));
    assert!(!b.contains("<kbd>") && b.contains("&lt;kbd&gt;"), "{b}");
}

#[test]
fn comments_are_dropped_and_entities_kept() {
    let b = html("a <!-- secret --> b &copy; &amp; &bogus x\n");
    assert!(!b.contains("secret"), "{b}");
    assert!(b.contains("\u{a9}") && b.contains("&amp;bogus"), "{b}");
    let raw = html("<p>&copy; &#65;</p>\n");
    assert!(raw.contains("&copy; &#65;"), "{raw}");
}

#[test]
fn attribute_and_class_values_cannot_break_out() {
    let p = page("[t](https://a.b/\"onmouseover=\"x \"title\")\n\n> [!a\"b<c] T\"<x\n> body");
    assert!(!p.body.contains("onmouseover=\"x"), "{}", p.body);
    assert!(!p.body.contains("<c"), "{}", p.body);
}

/// A tag checker: every non-void tag is closed, in order.
fn balanced(h: &str) -> bool {
    let mut stack: Vec<String> = Vec::new();
    let mut rest = h;
    while let Some(i) = rest.find('<') {
        let Some(j) = rest[i..].find('>') else {
            return false;
        };
        let tag = &rest[i + 1..i + j];
        rest = &rest[i + j + 1..];
        if let Some(name) = tag.strip_prefix('/') {
            if stack.pop().as_deref() != Some(name) {
                return false;
            }
        } else {
            let name = tag.split([' ', '/']).next().unwrap_or("").to_string();
            if !["br", "hr", "img", "input", "meta", "link"].contains(&name.as_str()) {
                stack.push(name);
            }
        }
    }
    stack.is_empty()
}

fn check(doc: &Document) {
    let h = fs_text_html::HtmlExporter::default().export(doc);
    let l = h.to_ascii_lowercase();
    assert!(
        !l.contains("<script") && !l.contains("<iframe") && !l.contains("<style"),
        "{h}"
    );
    assert!(
        !l.contains("href=\"javascript") && !l.contains("src=\"javascript"),
        "{h}"
    );
    assert!(!h.contains("\u{0}"), "NUL in output");
    assert!(balanced(&h), "not well formed: {h}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]
    #[test]
    fn any_markdown_gives_safe_well_formed_html(s in "[ -~\\n\\t]{0,300}") {
        check(&import_markdown(&s));
    }

    #[test]
    fn markdown_made_of_hostile_pieces_stays_safe(parts in prop::collection::vec(
        prop::sample::select(vec![
            "<script>", "</script>", "<kbd>", "</kbd>", "**", "*", "`", "[a](", "javascript:x", ")", "](", "![", "<details>", "</details>",
            "\n\n", "\n", "> ", "- [ ] ", "1. ", "| a | b |\n|--|--|\n", "[^1]", "[^1]: n", "<!--", "-->", "<img onerror=x>", "&amp;", "\"", "'",
        ]), 0..40)) {
        check(&import_markdown(&parts.concat()));
    }
}
