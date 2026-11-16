//! Page addresses: one message for every bad part, with a name that would do.
mod common;
use common::*;

#[test]
fn every_kind_of_bad_address_gets_the_same_message_and_a_suggestion() {
    for (file, bad, suggestion) in [
        ("Über uns.md", "Über uns", "ueber-uns"),
        ("Kontakt.md", "Kontakt", "kontakt"),
        ("cafe\u{301}.md", "cafe\u{301}", "cafe"),
        ("Straße/x.md", "Straße", "strasse"),
        ("a b/c.md", "a b", "a-b"),
    ] {
        let t = scratch(false);
        put(&t, &format!("content/en/{file}"), b"# x\n");
        let e = refused(&t);
        assert!(
            e.contains("must be lower case a-z 0-9 . _ -"),
            "{file}: {e}"
        );
        assert!(e.contains(&format!("{bad:?}")), "{file}: {e}");
        assert!(e.contains(&format!("{suggestion:?}")), "{file}: {e}");
        assert!(!e.contains("not allowed in a path"), "{file}: {e}");
    }
}

#[test]
fn nothing_is_renamed_and_all_problems_come_together() {
    let t = scratch(false);
    put(&t, "content/en/Über.md", b"# x\n");
    put(&t, "content/en/Zwei.md", b"# y\n");
    let e = refused(&t);
    assert!(e.contains("ueber") && e.contains("zwei"), "{e}");
}

#[test]
fn good_addresses_still_build() {
    let t = scratch(false);
    put(&t, "content/en/ueber-uns_2.v1.md", b"# x\n");
    assert!(builds(&t));
}
