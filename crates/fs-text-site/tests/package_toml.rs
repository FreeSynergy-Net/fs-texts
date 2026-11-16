//! The `site.toml` written into the output folder (the package root).
use fs_text_site::SiteConfig;

fn v2(extra_site: &str, names: &str) -> String {
    format!(
        "schema = 2\npurpose = \"website\"\n[site]\nid = \"a\"\n{extra_site}\n[names]\n{names}\n[acme]\nmethod = \"http-01\"\n"
    )
}

#[test]
fn schema2_output_keeps_id_prefix_languages_and_folder_rules() {
    let t = v2(
        "name = \"The \\\"Wiki\\\"\"\npath_prefix = \"/wiki\"\nlanguages = [\"de\", \"en\"]\ndefault_language = \"en\"",
        "primary = \"example.test\"\naliases = [\"www.example.test\"]",
    ) + "[dns]\nrecords = [\"A\", \"CAA\"]\n[csp]\nmode = \"static\"\n[build]\ndead_links = \"list\"\n[[csp.area]]\npath = \"/apps\"\nmode = \"passthrough\"\npolicy = \"default-src 'self'; script-src 'self'\"\nallow_loosen = true\n";
    let c = SiteConfig::parse(&t).unwrap();
    let out = c.to_package_toml();
    assert!(out.starts_with("schema = 2\n"), "{out}");
    for want in [
        "id = \"a\"",
        "name = \"The \\\"Wiki\\\"\"",
        "path_prefix = \"/wiki\"",
        "languages = [\"de\", \"en\"]",
        "default_language = \"en\"",
        "[[csp.area]]\npath = \"/apps\"\nmode = \"passthrough\"\npolicy = ",
    ] {
        assert!(out.contains(want), "{want} in {out}");
    }
    // what only the source knows does not go into the package (fs-release refuses it)
    assert!(
        !out.contains("[build]") && !out.contains("allow_loosen"),
        "{out}"
    );
    // a source without the loosening reads back the same
    let strict = t
        .replace("allow_loosen = true\n", "")
        .replace(
            "default-src 'self'; script-src 'self'",
            "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'",
        );
    let c = SiteConfig::parse(&strict).unwrap();
    let back = SiteConfig::parse(&c.to_package_toml()).unwrap();
    assert_eq!(
        (back.id.as_str(), back.name.as_str()),
        ("a", "The \"Wiki\"")
    );
    assert_eq!(back.path_prefix, c.path_prefix);
    assert_eq!(back.languages, c.languages);
    assert_eq!(back.default_language, c.default_language);
    assert_eq!(back.names, c.names);
    assert_eq!(back.csp_areas, c.csp_areas);
}

#[test]
fn schema1_source_stays_schema1_in_the_output() {
    let t = "schema = 1\npurpose = \"website\"\n[names]\nprimary = \"example.test\"\n[acme]\nmethod = \"http-01\"\n[csp]\nmode = \"passthrough\"\npolicy = \"default-src 'self'\"\n";
    let c = SiteConfig::parse(t).unwrap();
    let out = c.to_package_toml();
    assert!(
        out.starts_with("schema = 1\n") && !out.contains("[site]"),
        "{out}"
    );
    let back = SiteConfig::parse(&out).unwrap();
    assert_eq!(back.names, c.names);
    assert_eq!(back.csp_mode, c.csp_mode);
    assert_eq!(back.csp_policy, c.csp_policy);
}
