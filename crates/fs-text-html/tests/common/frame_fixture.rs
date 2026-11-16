use fs_text_html::{Alternate, Csp, Frame, NavNode, Navigation};

pub fn frame() -> Frame {
    let n = |t: &str, h: &str, c: Vec<NavNode>| NavNode { title: t.into(), href: h.into(), children: c };
    Frame {
        lang: "de".into(),
        site_name: "Example Site".into(),
        canonical: Some("https://example.test/de/guide/".into()),
        alternates: vec![
            Alternate { lang: "de".into(), href: "https://example.test/de/guide/".into() },
            Alternate { lang: "en".into(), href: "https://example.test/en/guide/".into() },
        ],
        x_default: Some("https://example.test/".into()),
        stylesheet: "/_fs/theme.css".into(),
        csp: Csp::Strict,
        nav: Navigation {
            roots: vec![
                n("Start", "/de/", vec![]),
                n("Guide", "/de/guide/", vec![n("Install", "/de/guide/install/", vec![]), n("Use", "/de/guide/use/", vec![])]),
                n("About", "/de/about/", vec![n("Team", "/de/about/team/", vec![])]),
            ],
        },
        current: "/de/guide/".into(),
        footer: Some("Example footer".into()),
    }
}

