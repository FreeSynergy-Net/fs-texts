//! Is one Content-Security-Policy looser than another?
//!
//! Pattern: Specification (a pure comparison). A folder of a site may carry its own policy, but
//! never a looser one than the site's unless the site.toml says so in so many words. The
//! comparison is conservative: whatever it cannot prove to be as strict counts as looser.
//!
//! The browser decides per effective directive, so this does too: a missing directive is
//! resolved along the CSP3 fallback chain (`worker-src` -> `child-src` -> `script-src` ->
//! `default-src`, ...) on BOTH sides before the source lists are compared.

use std::collections::{BTreeMap, BTreeSet};

/// Where a missing directive falls back to, in order (CSP3).
fn fallback(name: &str) -> &'static [&'static str] {
    match name {
        "script-src-elem" | "script-src-attr" => &["script-src", "default-src"],
        "style-src-elem" | "style-src-attr" => &["style-src", "default-src"],
        "worker-src" => &["child-src", "script-src", "default-src"],
        "frame-src" => &["child-src", "default-src"],
        "script-src" | "style-src" | "child-src" | "img-src" | "font-src" | "connect-src"
        | "media-src" | "object-src" | "manifest-src" | "prefetch-src" => &["default-src"],
        _ => &[],
    }
}

/// Directives that are always compared, even if neither policy names them.
const FETCH: [&str; 16] = [
    "default-src",
    "script-src",
    "script-src-elem",
    "script-src-attr",
    "style-src",
    "style-src-elem",
    "style-src-attr",
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

/// Directives without a fallback that the code knows. An area may add any of them. A directive
/// outside both lists that the site does not carry counts as looser (unknown = not provable).
const OTHER: [&str; 8] = [
    "base-uri",
    "form-action",
    "frame-ancestors",
    "sandbox",
    "upgrade-insecure-requests",
    "block-all-mixed-content",
    "require-trusted-types-for",
    "trusted-types",
];

/// What a directive allows: everything, or a set of sources.
#[derive(PartialEq, Eq)]
enum Allow {
    All,
    Set(BTreeSet<String>),
}

/// One source expression. Keywords, scheme and host are case-insensitive; hash and nonce values
/// and URL paths are not.
fn source(t: &str) -> String {
    if t.starts_with('\'') {
        let low = t.to_ascii_lowercase();
        for pre in ["'sha256-", "'sha384-", "'sha512-", "'nonce-"] {
            if low.starts_with(pre) {
                return format!("{pre}{}", &t[pre.len()..]);
            }
        }
        return low;
    }
    let from = t.find("://").map_or(0, |i| i + 3);
    let end = t[from..].find('/').map_or(t.len(), |i| from + i);
    format!("{}{}", t[..end].to_ascii_lowercase(), &t[end..])
}

fn parse(p: &str) -> BTreeMap<String, Vec<String>> {
    let mut m = BTreeMap::new();
    for d in p.split(';') {
        let mut it = d.split_whitespace();
        if let Some(name) = it.next() {
            // the first of a directive counts (a repeated one is ignored by browsers)
            m.entry(name.to_ascii_lowercase())
                .or_insert_with(|| it.map(source).collect());
        }
    }
    m
}

/// The effective source list of `name`: itself, else the first of its fallbacks that is set.
fn allowed(m: &BTreeMap<String, Vec<String>>, name: &str) -> Allow {
    let list = std::iter::once(name)
        .chain(fallback(name).iter().copied())
        .find_map(|n| m.get(n));
    match list {
        None => Allow::All,
        Some(l) => Allow::Set(l.iter().filter(|t| *t != "'none'").cloned().collect()),
    }
}

/// A hash, a nonce or `'strict-dynamic'`: its presence switches `'unsafe-inline'` off.
fn disarms(t: &str) -> bool {
    t.starts_with("'sha") || t.starts_with("'nonce-") || t == "'strict-dynamic'"
}

fn within(inner: &Allow, outer: &Allow) -> bool {
    match (inner, outer) {
        (_, Allow::All) => true,
        (Allow::All, Allow::Set(_)) => false,
        (Allow::Set(a), Allow::Set(b)) => {
            let guard = |s: &BTreeSet<String>| -> BTreeSet<String> {
                s.iter().filter(|t| disarms(t)).cloned().collect()
            };
            // Where the outer one has such values, `'unsafe-inline'` is dead there; losing
            // one of them could revive it, so the values must stay exactly the same.
            a.is_subset(b) && (guard(b).is_empty() || guard(a) == guard(b))
        }
    }
}

/// True if `area` allows more than `site` in any directive.
#[must_use]
pub fn looser(site: &str, area: &str) -> bool {
    let (s, a) = (parse(site), parse(area));
    if a.keys()
        .any(|n| !s.contains_key(n) && !FETCH.contains(&n.as_str()) && !OTHER.contains(&n.as_str()))
    {
        return true;
    }
    let mut names: BTreeSet<String> = s.keys().chain(a.keys()).cloned().collect();
    names.extend(FETCH.iter().map(|n| (*n).to_string()));
    names
        .iter()
        .any(|n| !within(&allowed(&a, n), &allowed(&s, n)))
}

#[cfg(test)]
mod tests;
