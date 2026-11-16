//! Counter-probes: injection through names, aliases and path prefixes, and breakout from a prefix.
use fs_text_site::{Host, PathPrefix, SiteConfig};

fn v2(extra_site: &str, names: &str) -> String {
    format!(
        "schema = 2\npurpose = \"website\"\n[site]\nid = \"a\"\n{extra_site}\n[names]\n{names}\n[acme]\nmethod = \"http-01\"\n"
    )
}

fn names(primary: &str) -> String {
    format!("primary = \"{primary}\"")
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
        let t = v2(
            "",
            &format!("primary = \"example.test\"\naliases = [{}]", toml_str(e)),
        );
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
fn path_prefix_breakout_is_refused() {
    let bad = [
        "wiki",
        "/wiki/",
        "//wiki",
        "/wiki//x",
        "/..",
        "/wiki/..",
        "/wiki/../x",
        "/./wiki",
        "/.hidden",
        "/%2e%2e",
        "/wiki%2fx",
        "/wi ki",
        "/WIKI",
        "/wiki\\x",
        "/wiki?x",
        "/wiki#x",
        "/_fs",
        "/_fs/x",
        "/a/b/c/d/e",
        "",
        "/wiki\n",
        "/wiki\0",
        "/\u{fe}x",
        "/wiki:x",
    ];
    for b in bad {
        assert!(PathPrefix::parse(b).is_err(), "must refuse {b:?}");
        let t = v2(
            &format!("path_prefix = {}", toml_str(b)),
            &names("example.test"),
        );
        assert!(SiteConfig::parse(&t).is_err(), "{b:?}");
    }
    for ok in ["/", "/wiki", "/a/b", "/v1.2", "/my_wiki-2"] {
        assert!(PathPrefix::parse(ok).is_ok(), "{ok}");
    }
    assert!(PathPrefix::parse("/").unwrap().is_root());
}
