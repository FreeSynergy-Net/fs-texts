//! Host names against the rules of fs-release: A-labels must be real Punycode, the last A-label
//! has two characters or more, a site name has no bidi or format character.
use fs_text_site::{ConfigError, Host, SiteConfig};

// Samples from the Python codec that fs-release uses (`bytes.decode("punycode")`, strict).
const DECODES: &[&str] = &[
    "0awqcuurtws",
    "0iid12qor",
    "0nfdbi1ds",
    "14khsn",
    "1hf",
    "1q85f",
    "25msx",
    "2awxcqnux",
    "2gjg3cict",
    "2gjwmr74i",
    "2gx4l64coiju",
    "2hmhqm",
    "2hqi935ka",
    "2tb145z",
    "2v6d95ya6x",
    "2vdvatpy",
    "2yseog",
    "33ogfsv",
    "34x03lhwq",
    "376jblj4hu7g",
    "3cu2u2qiprj",
    "3ilwb",
    "3mi56p6juv",
    "3ua",
    "3y1iltju",
    "3zghocktclo",
    "4lji7xy9ql8u",
    "5eo-ocmczo",
    "5kd",
    "5nbewd",
    "5nn43u",
    "5v0fvz3buvbj",
    "6deglv3l0c",
    "6ho",
    "6mdd-irabu",
    "6sbbgd",
    "71whsdhuwf",
    "776m",
    "787fqko6o",
    "7ht",
    "80ak6aa92e",
    "86hfry-afo",
    "8dbs3ltqd",
    "8gpgrsvacn",
    "8w3sr28h",
    "90j4gg766ya",
    "9ctlyr",
    "9uzfbt8y",
    "9v3nxgbbe5m",
    "a",
    "a6q",
    "abc",
    "awfh",
    "bcher-kva",
    "bd8kxw",
    "bm32g",
    "botzekbkjbcc",
    "c4d8j",
    "c4yk",
    "cev",
    "cgdfosiwlqk",
    "d0wgz",
    "d7h0zaj0fn",
    "d7ucs1xica",
    "de2v",
    "devvcute",
    "djdjd2uf",
    "dzcpcl3q5e",
    "e5e",
    "e6valka",
    "emv",
    "evfy0b314e5a",
    "exbcvg",
    "f5lmm7dw504r",
    "fbe2ynf7x",
    "fddanh1fmw",
    "fdhdzoi",
    "fiqs8s",
];
const DOES_NOT: &[&str] = &[
    "0",
    "04ivbo2jx0",
    "0o",
    "0ohugpk1b0lu",
    "1",
    "168hqo35dzv",
    "18",
    "18ztoiwi",
    "1dupkge2",
    "1ripsyiyr6",
    "2",
    "29f9pm",
    "2i",
    "2o4i-3q",
    "2syd5bh26",
    "3",
    "35",
    "353",
    "3ezfo6",
    "3h",
    "3hniz52r1rw",
    "3j4",
    "3t",
    "3t4bo8zwp",
    "4",
    "46u0xugfr9",
    "48m40zc",
    "4do4",
    "4epn3ysmjti",
    "4jat2",
    "5",
    "52",
    "53hfei9h6",
    "55fr8mrot",
    "566nh",
    "5enoraem0",
    "5g3",
    "5g524ei",
    "5j667b",
    "5k5zjyvv",
    "5rm6kwew",
    "6",
    "64qpczry",
    "6gdnokq17h-p",
    "6njj21qo6",
    "6ny1p6n43",
    "6ze38",
    "6zrp8",
    "75w9",
    "760",
    "7ktuwx",
    "7rdx6np",
    "7xl3cx",
    "7y1a8afc8u95",
    "82",
    "8aloh6yb1",
    "8dsan63nh",
    "8hc1",
    "8i",
    "8m",
    "8tlm9",
    "8v",
    "90rzsha-x",
    "94sqp9hk",
    "97sn6qmuyp87",
    "9f",
    "a0",
    "a7u6",
    "a927pa8",
    "a95jrh2",
    "a9ks8jn3f0",
    "af9jk8po0",
    "appawfy4",
    "aq58817",
    "awgi22dkp44",
    "azam98j0j6g7",
    "b",
    "b535hl",
    "b7",
    "bc6mp",
];

#[test]
fn xn_labels_decode_exactly_like_the_python_codec_of_fs_release() {
    // as the first label (not the last one, which has its own length rule)
    for l in DECODES {
        let h = format!("xn--{l}.example.test");
        assert!(Host::parse(&h).is_ok(), "{h} must be accepted");
    }
    for l in DOES_NOT {
        let h = format!("xn--{l}.example.test");
        assert!(Host::parse(&h).is_err(), "{h} must be refused");
    }
    // schema 1 does not decode (unchanged on both sides)
    assert!(Host::parse_v1("xn--zzzzzzzz.example.test").is_ok());
}

#[test]
fn the_last_a_label_has_at_least_two_characters_after_the_prefix() {
    assert!(Host::parse("example.xn--p1ai").is_ok());
    assert!(Host::parse("example.xn--ls8h").is_ok());
    assert!(
        Host::parse("example.xn--a").is_err(),
        "5 characters: fs-release wants 6"
    );
    assert!(Host::parse("example.xn--").is_err());
    // an A-label that is not Punycode is refused in any place
    assert!(Host::parse("xn--zzzzzzzz.example.test").is_err());
    assert!(Host::parse("example.xn--zzzzzzzz").is_err());
}

#[test]
fn a_site_name_may_not_hold_bidi_format_or_separator_characters() {
    let t = |n: &str| {
        format!("schema = 2\npurpose = \"website\"\n[site]\nid = \"a\"\nname = \"{n}\"\n[names]\nprimary = \"example.test\"\n[acme]\nmethod = \"http-01\"\n")
    };
    for bad in [
        "a\\u202Eb",
        "a\\u200Bb",
        "a\\uFEFFb",
        "a\\u2028b",
        "a\\u2029b",
        "a\\u00ADb",
        "a\\u2066b",
    ] {
        assert_eq!(
            SiteConfig::parse(&t(bad)).unwrap_err(),
            ConfigError::BadName,
            "{bad}"
        );
    }
    for ok in ["Wiki", "Küche und Café", "日本語", "a b"] {
        assert!(SiteConfig::parse(&t(ok)).is_ok(), "{ok}");
    }
}
