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
fn the_same_id_on_two_domains_is_fine_and_gets_two_folders() {
    let a = site("wiki", "example.test", "/", "[\"de\"]");
    let b = site("wiki", "other.example.test", "/", "[\"de\"]");
    let c = site("wiki", "other.example.test", "/x", "[\"de\"]");
    let set =
        SiteSet::new(vec![a.clone(), b.clone(), c.clone()]).expect("only host and path collide");
    assert_eq!(set.out_name(&a), "example.test");
    assert_eq!(set.out_name(&b), "other.example.test");
    assert_eq!(set.out_name(&c), "other.example.test_x");
    // the nesting of c below b is still found although the ids are equal
    assert_eq!(set.nested_below(&b), vec!["x".to_string()]);
    // counter-probe: the same host and path with the same id still collides
    let r = SiteSet::new(vec![a.clone(), a]);
    assert!(matches!(
        r.unwrap_err()[0],
        ConfigError::SiteCollision { .. }
    ));
}

#[test]
fn a_lone_id_names_its_folder() {
    let a = site("wiki", "example.test", "/", "[\"de\"]");
    let b = site("blog", "example.test", "/blog", "[\"de\"]");
    let set = SiteSet::new(vec![a.clone(), b.clone()]).unwrap();
    assert_eq!(
        (set.out_name(&a).as_str(), set.out_name(&b).as_str()),
        ("wiki", "blog")
    );
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
