//! Is one Content-Security-Policy looser than another?
//!
//! Pattern: Specification (a pure comparison). A folder of a site may carry its own policy, but
//! never a looser one than the site's unless the site.toml says so in so many words. The
//! comparison is conservative: whatever it cannot prove to be as strict counts as looser.

use std::collections::{BTreeMap, BTreeSet};

/// Directives that fall back to `default-src` when they are missing.
const FETCH: [&str; 11] = [
    "script-src",
    "style-src",
    "img-src",
    "font-src",
    "connect-src",
    "media-src",
    "object-src",
    "child-src",
    "frame-src",
    "worker-src",
    "manifest-src",
];

/// What a directive allows: nothing special, everything, or a set of sources.
#[derive(PartialEq, Eq)]
enum Allow {
    All,
    Set(BTreeSet<String>),
}

fn parse(p: &str) -> BTreeMap<String, Vec<String>> {
    let mut m = BTreeMap::new();
    for d in p.split(';') {
        let mut it = d.split_whitespace();
        if let Some(name) = it.next() {
            // the first of a directive counts (a repeated one is ignored by browsers)
            m.entry(name.to_ascii_lowercase())
                .or_insert_with(|| it.map(str::to_ascii_lowercase).collect());
        }
    }
    m
}

fn allowed(m: &BTreeMap<String, Vec<String>>, name: &str) -> Allow {
    let list = m.get(name).or_else(|| {
        if FETCH.contains(&name) || name == "default-src" {
            m.get("default-src")
        } else {
            None
        }
    });
    match list {
        None => Allow::All,
        Some(l) => Allow::Set(l.iter().filter(|t| *t != "'none'").cloned().collect()),
    }
}

fn within(inner: &Allow, outer: &Allow) -> bool {
    match (inner, outer) {
        (_, Allow::All) => true,
        (Allow::All, Allow::Set(_)) => false,
        (Allow::Set(a), Allow::Set(b)) => a.is_subset(b),
    }
}

/// True if `area` allows more than `site` in any directive.
#[must_use]
pub fn looser(site: &str, area: &str) -> bool {
    let (s, a) = (parse(site), parse(area));
    let mut names: BTreeSet<String> = s.keys().chain(a.keys()).cloned().collect();
    names.extend(FETCH.iter().map(|n| (*n).to_string()));
    names.insert("default-src".to_string());
    names
        .iter()
        .any(|n| !within(&allowed(&a, n), &allowed(&s, n)))
}

#[cfg(test)]
mod tests {
    use super::looser;

    const STRICT: &str = "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'";

    #[test]
    fn stricter_or_equal_is_fine() {
        assert!(!looser(STRICT, STRICT));
        assert!(!looser(
            STRICT,
            "default-src 'none'; base-uri 'none'; form-action 'none'"
        ));
        // a directive left out falls back to nothing for base-uri and form-action: looser
        assert!(looser(STRICT, "default-src 'none'"));
        assert!(!looser(STRICT, "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; img-src 'none'"));
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
}
