//! `site.toml`: schema 1 and 2, counter-tests against injection and breakout, and the checks
//! across sites. Host names are fixtures only.
use fs_text_site::{ConfigError, Host, PathPrefix, SiteConfig, SiteSet};

fn v2(extra_site: &str, names: &str) -> String {
    format!(
        "schema = 2\npurpose = \"website\"\n[site]\nid = \"a\"\n{extra_site}\n[names]\n{names}\n[acme]\nmethod = \"http-01\"\n"
    )
}

fn names(primary: &str) -> String {
    format!("primary = \"{primary}\"")
}

#[test]
fn v1_still_reads_and_gets_defaults() {
    let c = SiteConfig::parse(
        "schema = 1\npurpose = \"website\"\n[names]\nprimary = \"example.test\"\naliases = [\"www.example.test\"]\n[acme]\nmethod = \"http-01\"\n",
    )
    .unwrap();
    assert_eq!(c.schema, 1);
    assert_eq!(c.id, "main");
    assert_eq!(c.host.as_str(), "example.test");
    assert!(c.path_prefix.is_root());
    assert_eq!(c.names.len(), 2);
}

#[test]
fn v1_refuses_the_site_table_and_unknown_keys() {
    let t = "schema = 1\npurpose = \"website\"\n[site]\nid = \"a\"\n[names]\nprimary = \"example.test\"\n[acme]\nmethod = \"http-01\"\n";
    assert!(matches!(SiteConfig::parse(t), Err(ConfigError::SiteTable(_))));
    let t = "schema = 1\npurpose = \"website\"\ntoken = \"x\"\n[names]\nprimary = \"example.test\"\n[acme]\nmethod = \"http-01\"\n";
    assert!(matches!(SiteConfig::parse(t), Err(ConfigError::Toml(_))));
}

#[test]
fn v2_reads_all_fields() {
    let t = v2(
        "name = \"Wiki\"\nhost = \"www.example.test\"\npath_prefix = \"/wiki\"\nlanguages = [\"de\", \"en\"]\ndefault_language = \"en\"",
        "primary = \"example.test\"\naliases = [\"www.example.test\"]",
    );
    let c = SiteConfig::parse(&t).unwrap();
    assert_eq!(c.host.as_str(), "www.example.test");
    assert_eq!(c.url_prefix(), "/wiki");
    assert_eq!(c.default_language.as_str(), "en");
    assert_eq!(c.origin(), "https://www.example.test/wiki");
}

#[test]
fn v2_needs_a_site_table_and_unknown_key_is_refused() {
    let t = "schema = 2\npurpose = \"website\"\n[names]\nprimary = \"example.test\"\n[acme]\nmethod = \"http-01\"\n";
    assert!(matches!(SiteConfig::parse(t), Err(ConfigError::SiteTable(_))));
    let t = v2("colour = \"red\"", &names("example.test"));
    assert!(matches!(SiteConfig::parse(&t), Err(ConfigError::Toml(_))));
}

#[test]
fn unknown_schema_and_dns01_and_big_file() {
    assert_eq!(
        SiteConfig::parse("schema = 3\n").unwrap_err(),
        ConfigError::UnknownSchema(3)
    );
    let t = v2("", &names("example.test")).replace("http-01", "dns-01");
    assert!(matches!(SiteConfig::parse(&t), Err(ConfigError::AcmeMethod(_))));
    let t = format!("{}\n# {}", v2("", &names("example.test")), "x".repeat(9000));
    assert_eq!(SiteConfig::parse(&t).unwrap_err(), ConfigError::TooBig);
}

/// KDL and log injection through names and aliases: every one of these is refused, so the
/// Zentinel configuration never sees a quote, brace, semicolon, backslash or line break.
#[test]
fn hosts_with_kdl_or_log_injection_are_refused() {
    let evil = [
        "a.test\"; route { }",
        "a.test\\\"",
        "a.test}",
        "a.test{",
        "a.test;",
        "a.test\n[names]",
        "a.test\r",
        "a.test\0",
        "a b.test",
        "a.test/",
        "a.test:8080",
        "user@a.test",
        "*.a.test",
        "A.test",
        "a.test.",
        "a..test",
        "-a.test",
        "a-.test",
        "localhost",
        "a.t",
        "a.123",
        "a.te-st",
        "bücher.test",
        "a.test#",
        "a.test?x",
        "a.test$(id)",
        "a.test`id`",
        "a.test'",
        "a.test\u{202e}",
        "a.test\u{200b}",
    ];
    for e in evil {
        assert!(Host::parse(e).is_err(), "must refuse {e:?}");
        let t = v2("", &format!("primary = \"example.test\"\naliases = [{}]", toml_str(e)));
        assert!(SiteConfig::parse(&t).is_err(), "alias {e:?}");
        let t = v2("", &format!("primary = {}", toml_str(e)));
        assert!(SiteConfig::parse(&t).is_err(), "primary {e:?}");
    }
}

fn toml_str(s: &str) -> String {
    // a TOML basic string with every special character escaped, so the file itself stays valid
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\0' => o.push_str("\\u0000"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[test]
fn good_hosts_are_accepted() {
    for h in ["example.test", "a.b.example.test", "xn--bcher-kva.example.test", "a.xn--p1ai", "a-b.example.org"] {
        assert!(Host::parse(h).is_ok(), "{h}");
    }
    // the last label as an A-label is new in schema 2; schema 1 stays narrow
    assert!(Host::parse_v1("a.xn--p1ai").is_err());
}

#[test]
fn names_rules_duplicates_and_limits() {
    let dup = "primary = \"example.test\"\naliases = [\"example.test\"]";
    assert!(matches!(SiteConfig::parse(&v2("", dup)), Err(ConfigError::Names(_))));
    let dup = "primary = \"example.test\"\naliases = [\"a.example.test\", \"a.example.test\"]";
    assert!(matches!(SiteConfig::parse(&v2("", dup)), Err(ConfigError::Names(_))));
    let many: Vec<String> = (0..10).map(|i| format!("\"a{i}.example.test\"")).collect();
    let t = v2("", &format!("primary = \"example.test\"\naliases = [{}]", many.join(",")));
    assert!(matches!(SiteConfig::parse(&t), Err(ConfigError::Names(_))));
}

#[test]
fn host_must_be_one_of_the_names() {
    let t = v2("host = \"other.example.test\"", &names("example.test"));
    assert!(matches!(
        SiteConfig::parse(&t),
        Err(ConfigError::HostNotInNames(_))
    ));
}

#[test]
fn path_prefix_breakout_is_refused() {
    let bad = [
        "wiki", "/wiki/", "//wiki", "/wiki//x", "/..", "/wiki/..", "/wiki/../x", "/./wiki",
        "/.hidden", "/%2e%2e", "/wiki%2fx", "/wi ki", "/WIKI", "/wiki\\x", "/wiki?x", "/wiki#x",
        "/_fs", "/_fs/x", "/a/b/c/d/e", "", "/wiki\n", "/wiki\0", "/\u{fe}x", "/wiki:x",
    ];
    for b in bad {
        assert!(PathPrefix::parse(b).is_err(), "must refuse {b:?}");
        let t = v2(&format!("path_prefix = {}", toml_str(b)), &names("example.test"));
        assert!(SiteConfig::parse(&t).is_err(), "{b:?}");
    }
    for ok in ["/", "/wiki", "/a/b", "/v1.2", "/my_wiki-2"] {
        assert!(PathPrefix::parse(ok).is_ok(), "{ok}");
    }
    assert!(PathPrefix::parse("/").unwrap().is_root());
}

#[test]
fn ids_names_and_languages() {
    for bad in ["", "A", "-a", "a-", "a b", "a/b", "../x", &"a".repeat(33)] {
        let t = v2(&format!("x = 1"), &names("example.test")).replace("id = \"a\"", &format!("id = {}", toml_str(bad)));
        let t = t.replace("x = 1\n", "");
        assert!(matches!(SiteConfig::parse(&t), Err(ConfigError::BadId(_))), "{bad:?}");
    }
    for bad in ["[\"DE\"]", "[\"d\"]", "[\"de\", \"de\"]", "[]", "[\"../x\"]", "[\"de/en\"]", "[\"_fs\"]", "[\"a\",\"b\",\"c\",\"d\",\"e\",\"f\",\"g\",\"h\",\"i\"]"] {
        let t = v2(&format!("languages = {bad}"), &names("example.test"));
        assert!(SiteConfig::parse(&t).is_err(), "{bad}");
    }
    let t = v2("languages = [\"de\"]\ndefault_language = \"en\"", &names("example.test"));
    assert!(matches!(SiteConfig::parse(&t), Err(ConfigError::DefaultNotListed(_))));
    let t = v2("name = \"a\\nb\"", &names("example.test"));
    assert_eq!(SiteConfig::parse(&t).unwrap_err(), ConfigError::BadName);
    let t = v2("languages = [\"de\", \"pt-br\", \"zh-hant\"]", &names("example.test"));
    assert!(SiteConfig::parse(&t).is_ok());
}

#[test]
fn dns_acme_csp_rules_of_schema_1_hold() {
    let base = |extra: &str| format!("{}{extra}", v2("", &names("example.test")));
    assert!(SiteConfig::parse(&base("[dns]\nrecords = [\"A\", \"CAA\"]\n")).is_ok());
    for bad in ["[dns]\nrecords = []\n", "[dns]\nrecords = [\"A\", \"A\"]\n", "[dns]\nrecords = [\"MX\"]\n"] {
        assert!(matches!(SiteConfig::parse(&base(bad)), Err(ConfigError::DnsRecords(_))), "{bad}");
    }
    assert!(matches!(
        SiteConfig::parse(&base("[csp]\nmode = \"loose\"\n")),
        Err(ConfigError::CspMode(_))
    ));
    for bad in ["a\\\"b", "a@b", "a&b", "a\\nb", "a\\\\b", ""] {
        let t = base(&format!("[csp]\npolicy = \"{bad}\"\n"));
        assert_eq!(SiteConfig::parse(&t).unwrap_err(), ConfigError::CspPolicy, "{bad}");
    }
    assert!(SiteConfig::parse(&base("[csp]\npolicy = \"default-src 'self'\"\n")).is_ok());
}

#[test]
fn schema1_output_reads_back_the_same_and_has_no_site_table() {
    let t = v2(
        "path_prefix = \"/wiki\"\nlanguages = [\"de\"]",
        "primary = \"example.test\"\naliases = [\"www.example.test\"]",
    ) + "[dns]\nrecords = [\"A\", \"CAA\"]\n[csp]\nmode = \"passthrough\"\npolicy = \"default-src 'self'\"\n";
    let c = SiteConfig::parse(&t).unwrap();
    let out = c.to_schema1();
    assert!(!out.contains("[site]"));
    let back = SiteConfig::parse(&out).unwrap();
    assert_eq!(back.names, c.names);
    assert_eq!(back.dns_records, c.dns_records);
    assert_eq!(back.csp_mode, c.csp_mode);
    assert_eq!(back.csp_policy, c.csp_policy);
}

fn site(id: &str, host: &str, prefix: &str, langs: &str) -> SiteConfig {
    SiteConfig::parse(&v2(
        &format!("path_prefix = \"{prefix}\"\nlanguages = {langs}").replace("id = \"a\"", ""),
        &names(host),
    ).replace("id = \"a\"", &format!("id = \"{id}\"")))
    .unwrap()
}

#[test]
fn two_sites_on_the_same_host_and_path_collide() {
    let r = SiteSet::new(vec![
        site("one", "example.test", "/", "[\"de\"]"),
        site("two", "example.test", "/", "[\"en\"]"),
    ]);
    assert!(matches!(r.unwrap_err()[0], ConfigError::SiteCollision { .. }));
    let r = SiteSet::new(vec![
        site("one", "example.test", "/w", "[\"de\"]"),
        site("two", "example.test", "/w", "[\"en\"]"),
    ]);
    assert!(r.is_err());
}

#[test]
fn an_alias_of_one_site_that_is_the_host_of_another_collides() {
    let a = SiteConfig::parse(&v2("", "primary = \"example.test\"\naliases = [\"w.example.test\"]")).unwrap();
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
    for seg in ["de", "_fs".trim_start_matches('_'), "sitemap.xml", "robots.txt", "404.html", "index.html", "site.toml", "site-manifest.json"] {
        if seg == "fs" {
            continue;
        }
        let r = SiteSet::new(vec![
            site("outer", "example.test", "/", "[\"de\"]"),
            site("inner", "example.test", &format!("/{seg}"), "[\"en\"]"),
        ]);
        assert!(
            r.as_ref().is_err_and(|e| matches!(e[0], ConfigError::NestedShadowed { .. })),
            "{seg}: {r:?}"
        );
    }
}
