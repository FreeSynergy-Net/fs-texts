//! Tests of the comparison of two policies. Pattern: none (tests).

use super::looser;

const STRICT: &str =
    "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'";

#[test]
fn stricter_or_equal_is_fine() {
    assert!(!looser(STRICT, STRICT));
    assert!(!looser(
        STRICT,
        "default-src 'none'; base-uri 'none'; form-action 'none'"
    ));
    // a directive left out falls back to nothing for base-uri and form-action: looser
    assert!(looser(STRICT, "default-src 'none'"));
    assert!(!looser(STRICT, &format!("{STRICT}; img-src 'none'")));
    assert!(!looser(
        STRICT,
        &format!("{STRICT}; script-src-elem 'none'")
    ));
    assert!(!looser(STRICT, &format!("{STRICT}; worker-src 'none'")));
}

#[test]
fn anything_that_allows_more_is_looser() {
    assert!(looser(STRICT, "default-src 'self'; script-src 'self'"));
    assert!(looser(STRICT, "default-src 'self' https://x.test"));
    assert!(looser(STRICT, "default-src *"));
    assert!(looser(STRICT, "script-src 'self'"));
    assert!(looser(STRICT, "default-src 'self'; base-uri 'self'"));
    assert!(looser(STRICT, "img-src 'self' data:"));
    assert!(looser(STRICT, ""));
}

// ---- the six ways round the old guard (review P1), each with a counter-test -----------

#[test]
fn p1_script_src_elem_falls_back_to_script_src() {
    assert!(looser(STRICT, &format!("{STRICT}; script-src-elem 'self'")));
    assert!(!looser(
        STRICT,
        &format!("{STRICT}; script-src-elem 'none'")
    ));
    // where the site itself opens the element rule, the area may use it
    let site = format!("{STRICT}; script-src-elem 'self'");
    assert!(!looser(&site, &site));
}

#[test]
fn p2_script_src_attr_falls_back_to_script_src() {
    assert!(looser(
        STRICT,
        &format!("{STRICT}; script-src-attr 'unsafe-inline'")
    ));
    assert!(!looser(
        STRICT,
        &format!("{STRICT}; script-src-attr 'none'")
    ));
}

#[test]
fn p3_style_src_attr_falls_back_to_style_src() {
    let site = format!("{STRICT}; style-src 'self'");
    assert!(looser(
        &site,
        &format!("{site}; style-src-attr 'unsafe-inline'")
    ));
    assert!(looser(
        &site,
        &format!("{site}; style-src-elem https://x.test")
    ));
    assert!(!looser(&site, &format!("{site}; style-src-attr 'none'")));
    assert!(!looser(&site, &format!("{site}; style-src-elem 'self'")));
}

#[test]
fn p4_worker_src_goes_child_then_script_then_default() {
    assert!(looser(STRICT, &format!("{STRICT}; worker-src 'self'")));
    assert!(!looser(STRICT, &format!("{STRICT}; worker-src 'none'")));
    // with child-src in the site, that one rules (no jump to script-src)
    let site = "default-src 'none'; script-src 'none'; child-src 'self'; base-uri 'none'; form-action 'none'";
    assert!(!looser(site, &format!("{site}; worker-src 'self'")));
    assert!(looser(
        site,
        &format!("{site}; worker-src 'self' https://x.test")
    ));
}

#[test]
fn p5_frame_src_goes_child_then_default() {
    let site = "default-src 'self'; child-src 'none'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'";
    assert!(looser(site, &format!("{site}; frame-src 'self'")));
    assert!(!looser(site, &format!("{site}; frame-src 'none'")));
}

const HASHED: &str = "default-src 'none'; script-src 'self' 'unsafe-inline' 'sha256-AbC='; base-uri 'none'; form-action 'none'";

#[test]
fn p6_dropping_a_hash_revives_unsafe_inline() {
    let no_hash = HASHED.replace(" 'sha256-AbC='", "");
    assert!(looser(HASHED, &no_hash));
    // dropping a nonce or 'strict-dynamic' is the same
    let nonce = HASHED.replace("'sha256-AbC='", "'nonce-xYz'");
    assert!(looser(&nonce, &nonce.replace(" 'nonce-xYz'", "")));
    let dynamic = HASHED.replace("'sha256-AbC='", "'strict-dynamic'");
    assert!(looser(&dynamic, &dynamic.replace(" 'strict-dynamic'", "")));
    // counter-tests: the same values are fine, and where the site has none there is no guard
    assert!(!looser(HASHED, HASHED));
    assert!(!looser(&nonce, &nonce));
    let plain = HASHED.replace(" 'sha256-AbC='", "");
    assert!(!looser(&plain, &plain));
}

#[test]
fn hashes_and_paths_keep_their_case_keywords_do_not() {
    let site = "default-src 'none'; script-src 'sha256-AbC='; base-uri 'none'; form-action 'none'";
    assert!(looser(site, &site.replace("AbC", "abc")));
    assert!(looser(site, &site.replace("sha256-AbC=", "nonce-AbC")));
    assert!(!looser(site, &site.replace("sha256", "SHA256")));
    let site =
        "default-src 'none'; img-src https://X.test/Img; base-uri 'none'; form-action 'none'";
    assert!(looser(site, &site.replace("/Img", "/img")));
    assert!(!looser(
        site,
        &site.replace("https://X.test", "HTTPS://x.TEST")
    ));
    assert!(!looser(STRICT, &STRICT.replace("'self'", "'SELF'")));
}

#[test]
fn an_unknown_directive_the_site_does_not_carry_is_not_provable() {
    assert!(looser(STRICT, &format!("{STRICT}; navigate-to 'self'")));
    assert!(!looser(
        STRICT,
        &format!("{STRICT}; frame-ancestors 'none'")
    ));
    assert!(!looser(STRICT, &format!("{STRICT}; sandbox")));
}
