//! The links that must be on every page (contents, sitemap) are outside the budget, and a cut
//! says so.

use fs_text_html::{NavNode, Navigation};

fn node(t: &str, h: &str, children: Vec<NavNode>) -> NavNode {
    NavNode {
        title: t.into(),
        href: h.into(),
        children,
    }
}

fn flat(n: usize, with_extra: bool) -> Navigation {
    Navigation {
        roots: (0..n)
            .map(|i| node(&format!("P{i}"), &format!("/en/p{i}/"), vec![]))
            .collect(),
        extra: if with_extra {
            vec![
                node("Contents", "/en/contents/", vec![]),
                node("Sitemap", "/sitemap.xml", vec![]),
            ]
        } else {
            Vec::new()
        },
        more: "more".into(),
    }
}

#[test]
fn contents_and_sitemap_are_listed_whatever_the_top_level_holds() {
    for n in [3, 39, 40, 41, 45, 500] {
        let h = flat(n, true).render("/en/p0/");
        assert!(h.contains("href=\"/en/contents/\""), "{n}: {h}");
        assert!(h.contains("href=\"/sitemap.xml\""), "{n}: {h}");
    }
}

#[test]
fn a_cut_list_says_so_and_leads_to_the_contents() {
    let h = flat(45, true).render("/en/p0/");
    assert!(
        h.contains("<li class=\"more\"><a href=\"/en/contents/\">more</a></li>"),
        "{h}"
    );
    assert!(!h.contains("P44"), "{h}");
    let h = flat(40, true).render("/en/p0/");
    assert!(!h.contains("class=\"more\""), "nothing was cut: {h}");
}

#[test]
fn a_cut_list_in_a_folder_of_72_pages_has_the_hint_too() {
    let kids: Vec<NavNode> = (0..72)
        .map(|i| node(&format!("F{i}"), &format!("/en/a/f{i}/"), vec![]))
        .collect();
    let nav = Navigation {
        roots: vec![node("A", "/en/a/", kids)],
        extra: vec![node("Contents", "/en/contents/", vec![])],
        more: String::new(),
    };
    let h = nav.render("/en/a/f60/");
    assert!(h.contains("aria-current"), "{h}");
    assert!(h.contains("href=\"/en/contents/\">…</a>"), "{h}");
    assert!(h.contains("F39") && !h.contains("F41\""), "{h}");
    assert_eq!(h.matches("class=\"more\"").count(), 1, "{h}");
}

#[test]
fn without_an_extra_link_the_hint_is_plain_text() {
    let h = flat(45, false).render("/en/p0/");
    assert!(h.contains("<li class=\"more\">more</li>"), "{h}");
}
