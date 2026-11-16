//! The checks across sites: collisions, nesting, reserved names.
use fs_text_site::{ConfigError, SiteConfig, SiteSet};

fn v2(extra_site: &str, names: &str) -> String {
    format!(
        "schema = 2\npurpose = \"website\"\n[site]\nid = \"a\"\n{extra_site}\n[names]\n{names}\n[acme]\nmethod = \"http-01\"\n"
    )
}

fn names(primary: &str) -> String {
    format!("primary = \"{primary}\"")
}

fn site(id: &str, host: &str, prefix: &str, langs: &str) -> SiteConfig {
    SiteConfig::parse(
        &v2(
            &format!("path_prefix = \"{prefix}\"\nlanguages = {langs}").replace("id = \"a\"", ""),
            &names(host),
        )
        .replace("id = \"a\"", &format!("id = \"{id}\"")),
    )
    .unwrap()
}

#[test]
fn two_sites_on_the_same_host_and_path_collide() {
    let r = SiteSet::new(vec![
        site("one", "example.test", "/", "[\"de\"]"),
        site("two", "example.test", "/", "[\"en\"]"),
    ]);
    assert!(matches!(
        r.unwrap_err()[0],
        ConfigError::SiteCollision { .. }
    ));
    let r = SiteSet::new(vec![
        site("one", "example.test", "/w", "[\"de\"]"),
        site("two", "example.test", "/w", "[\"en\"]"),
    ]);
    assert!(r.is_err());
}

#[test]
fn an_alias_of_one_site_that_is_the_host_of_another_collides() {
    let a = SiteConfig::parse(&v2(
        "",
        "primary = \"example.test\"\naliases = [\"w.example.test\"]",
    ))
    .unwrap();
    let mut b = site("b", "w.example.test", "/", "[\"de\"]");
    b.id = "b".into();
    assert!(SiteSet::new(vec![a, b]).is_err());
}

#[test]
fn same_id_twice_is_refused() {
    let r = SiteSet::new(vec![
        site("one", "example.test", "/", "[\"de\"]"),
        site("one", "other.example.test", "/", "[\"de\"]"),
    ]);
    assert!(matches!(r.unwrap_err()[0], ConfigError::DuplicateId(_)));
}

#[test]
fn different_hosts_and_disjoint_paths_are_fine_and_nesting_is_reserved() {
    let ok = SiteSet::new(vec![
        site("one", "example.test", "/", "[\"de\"]"),
        site("two", "wiki.example.test", "/", "[\"de\"]"),
        site("three", "example.test", "/wiki", "[\"de\"]"),
        site("four", "example.test", "/blog", "[\"de\"]"),
    ])
    .unwrap();
    assert_eq!(ok.nested_below(&ok.sites()[0]), vec!["blog", "wiki"]);
    assert!(ok.nested_below(&ok.sites()[1]).is_empty());
    assert!(ok.nested_below(&ok.sites()[2]).is_empty());
}

#[test]
fn an_inner_site_on_a_generated_name_is_refused() {
    for seg in [
        "de",
        "_fs".trim_start_matches('_'),
        "sitemap.xml",
        "robots.txt",
        "404.html",
        "index.html",
        "site.toml",
        "site-manifest.json",
    ] {
        if seg == "fs" {
            continue;
        }
        let r = SiteSet::new(vec![
            site("outer", "example.test", "/", "[\"de\"]"),
            site("inner", "example.test", &format!("/{seg}"), "[\"en\"]"),
        ]);
        assert!(
            r.as_ref()
                .is_err_and(|e| matches!(e[0], ConfigError::NestedShadowed { .. })),
            "{seg}: {r:?}"
        );
    }
}
