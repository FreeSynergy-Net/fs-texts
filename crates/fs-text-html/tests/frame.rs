//! The page frame: accessibility basics, no external resource, short navigation, determinism.
mod common;
mod golden_frame {
    include!("common/frame_fixture.rs");
}
use common::page;
use fs_text_html::theme::CSS;
use fs_text_html::{render_page, Csp, NavNode, Navigation};
use golden_frame::frame;

fn rendered() -> String {
    render_page(&frame(), &page("---\ntitle: T\n---\n## Sub\n\ntext"))
}

#[test]
fn page_has_the_basics() {
    let h = rendered();
    assert!(h.starts_with("<!DOCTYPE html>\n<html lang=\"de\">"));
    assert!(h.contains("<meta charset=\"utf-8\">") && h.contains("name=\"viewport\""));
    assert!(h.contains("<title>T \u{2013} Example Site</title>"));
    assert!(h.contains("<a class=\"skip\" href=\"#main\">Zum Inhalt springen</a>"));
    assert_eq!(
        h.matches("<h1").count(),
        1,
        "front matter title becomes the one h1: {h}"
    );
    assert!(h.contains("<main id=\"main\">") && h.contains("<header>") && h.contains("<footer>"));
    assert!(h.contains("rel=\"canonical\"") && h.contains("hreflang=\"x-default\""));
    assert!(h.contains("hreflang=\"en\" href=\"https://example.test/en/guide/\""));
    assert!(h.find("charset").unwrap() < 1024);
}

#[test]
fn page_loads_nothing_from_outside_and_has_no_script() {
    let h = rendered();
    assert!(!h.to_ascii_lowercase().contains("<script"));
    for line in h.lines().filter(|l| {
        l.starts_with("<link rel=\"stylesheet\"") || l.contains("<img") || l.contains("<iframe")
    }) {
        assert!(
            !line.contains("://") && !line.contains("href=\"//"),
            "{line}"
        );
    }
    assert!(h.contains("default-src &#39;self&#39;") && h.contains("script-src &#39;none&#39;"));
    assert!(
        !CSS.contains("@import") && !CSS.contains("url(") && !CSS.contains("http"),
        "style sheet loads nothing"
    );
}

#[test]
fn csp_is_a_setting() {
    let mut f = frame();
    f.csp = Csp::Off;
    assert!(!render_page(&f, &page("# x")).contains("Content-Security-Policy"));
    f.csp = Csp::Custom("default-src 'self'; img-src https:".into());
    assert!(render_page(&f, &page("# x")).contains("img-src https:"));
}

#[test]
fn language_bar_only_with_two_variants() {
    let mut f = frame();
    assert!(render_page(&f, &page("# x")).contains("class=\"langs\""));
    f.alternates.truncate(1);
    assert!(!render_page(&f, &page("# x")).contains("class=\"langs\""));
}

#[test]
fn page_with_its_own_h1_gets_no_second_one() {
    let h = render_page(&frame(), &page("# Mine\n\ntext"));
    assert_eq!(h.matches("<h1").count(), 1, "{h}");
}

fn big() -> Navigation {
    let leaf = |a: usize, b: usize, c: usize| NavNode {
        title: format!("Page {a}.{b}.{c}"),
        href: format!("/de/{a}/{b}/{c}/"),
        children: vec![],
    };
    let mid = |a: usize, b: usize| NavNode {
        title: format!("Section {a}.{b}"),
        href: format!("/de/{a}/{b}/"),
        children: (0..20).map(|c| leaf(a, b, c)).collect(),
    };
    Navigation {
        roots: (0..20)
            .map(|a| NavNode {
                title: format!("Part {a}"),
                href: format!("/de/{a}/"),
                children: (0..20).map(|b| mid(a, b)).collect(),
            })
            .collect(),
    }
}

#[test]
fn navigation_is_short_whatever_the_size_of_the_tree() {
    let nav = big();
    for cur in ["/de/", "/de/3/", "/de/3/7/", "/de/3/7/9/", "/de/nowhere/"] {
        let h = nav.render(cur);
        assert!(h.len() <= 20 * 1024, "{cur}: {} bytes", h.len());
    }
    let h = nav.render("/de/3/7/");
    assert!(
        h.contains("Part 19") && h.contains("Part 3"),
        "top level is complete"
    );
    assert!(
        h.contains("Section 3.7") && h.contains("Section 3.8"),
        "siblings are there"
    );
    assert!(
        h.contains("Page 3.7.0") && h.contains("Page 3.7.19"),
        "children are there"
    );
    assert!(!h.contains("Section 4.0"), "no other branch");
    assert!(!h.contains("Page 3.8."), "no children of siblings");
    assert_eq!(h.matches("aria-current=\"page\"").count(), 1);
    // an ancestor shows only the next step of the path, not its other children
    let h = nav.render("/de/3/7/9/");
    assert!(
        h.contains("Section 3.7") && !h.contains("Section 3.8"),
        "{h}"
    );
    assert!(
        h.contains("Page 3.7.9") && h.contains("Page 3.7.10"),
        "siblings of the page"
    );
}

#[test]
fn trail_and_breadcrumb() {
    let nav = big();
    let t = nav.trail("/de/3/7/9/");
    assert_eq!(
        t.iter().map(|x| x.0).collect::<Vec<_>>(),
        ["Part 3", "Section 3.7", "Page 3.7.9"]
    );
    assert!(nav.trail("/de/x/").is_empty());
    let mut f = frame();
    f.nav = nav;
    f.current = "/de/3/7/".into();
    let h = render_page(&f, &page("# x"));
    assert!(
        h.contains("<nav class=\"crumbs\"")
            && h.contains("<li aria-current=\"page\">Section 3.7</li>"),
        "{h}"
    );
}

#[test]
fn same_input_same_bytes() {
    assert_eq!(rendered(), rendered());
    let a = page(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/rich.md"))
            .unwrap(),
    );
    let b = page(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/rich.md"))
            .unwrap(),
    );
    assert_eq!(
        a.body, b.body,
        "ids and numbers do not depend on a clock or random ids"
    );
}

#[test]
fn the_navigation_budget_is_kept_by_the_code_also_for_a_flat_folder() {
    use fs_text_html::{NavNode, Navigation};
    let kids: Vec<NavNode> = (0..5000)
        .map(|i| NavNode {
            title: format!("Page {i}"),
            href: format!("/en/a/p{i}/"),
            children: vec![],
        })
        .collect();
    let nav = Navigation {
        roots: vec![NavNode {
            title: "A".into(),
            href: "/en/a/".into(),
            children: kids,
        }],
    };
    for cur in ["/en/a/", "/en/a/p0/", "/en/a/p4999/"] {
        let h = nav.render(cur);
        assert!(h.len() < 20 * 1024, "{cur}: {} bytes", h.len());
        if cur.ends_with("p4999/") {
            assert!(
                h.contains("aria-current"),
                "the page itself is always listed"
            );
        }
    }
}
