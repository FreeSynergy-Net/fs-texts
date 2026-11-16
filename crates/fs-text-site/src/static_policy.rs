//! The policy a page of a purely static site may carry.
//!
//! Pattern: Specification. The release check of fs-release (`site-pruefen.py`, `csp_bad`)
//! refuses a generated page whose own policy is looser than the exporter's strict one, so the
//! build refuses it earlier, with the same rule: known directives only, each once, values only
//! `'self'` or `'none'`, a `default-src`, and `script-src`, `object-src`, `base-uri` and
//! `form-action` exactly `'none'`. In a passthrough site the HTML of the user is not judged, so
//! this does not apply there.

const KNOWN: [&str; 14] = [
    "default-src",
    "script-src",
    "style-src",
    "img-src",
    "font-src",
    "connect-src",
    "object-src",
    "base-uri",
    "form-action",
    "media-src",
    "manifest-src",
    "frame-src",
    "child-src",
    "worker-src",
];
const NONE: [&str; 4] = ["script-src", "object-src", "base-uri", "form-action"];

/// Why `policy` is not allowed in a static site, or `None`.
#[must_use]
pub(crate) fn bad(policy: &str) -> Option<String> {
    let mut seen: Vec<(String, Vec<&str>)> = Vec::new();
    for part in policy.split(';') {
        let mut it = part.split_whitespace();
        let Some(name) = it.next() else { continue };
        let name = name.to_ascii_lowercase();
        let vals: Vec<&str> = it.collect();
        if !KNOWN.contains(&name.as_str()) || seen.iter().any(|(n, _)| *n == name) {
            return Some(format!("the directive {name:?} is unknown or twice"));
        }
        if vals.is_empty() || vals.iter().any(|v| !matches!(*v, "'self'" | "'none'")) {
            return Some(format!("the value of {name} must be 'self' or 'none' only"));
        }
        seen.push((name, vals));
    }
    if !seen.iter().any(|(n, _)| n == "default-src") {
        return Some("there is no default-src".to_string());
    }
    for d in NONE {
        if !seen.iter().any(|(n, v)| n == d && v == &["'none'"]) {
            return Some(format!("{d} must be 'none' (a static site runs no script)"));
        }
    }
    None
}
