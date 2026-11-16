//! Which addresses may become a link or an image.
//!
//! Pattern: Specification — a pure predicate over an address string.

/// Is `raw` a link target we write? Allowed: `https:`, `http:`, `mailto:`, and relative
/// addresses (path, `?query`, `#fragment`). Everything else (`javascript:`, `data:`,
/// `vbscript:`, `file:`, unknown schemes), a protocol-relative `//host`, a backslash and any
/// control character (browsers drop tabs and line breaks inside a scheme) is refused.
#[must_use]
pub fn safe_link(raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() || s.contains('\\') || s.chars().any(|c| c.is_control() || c == '\u{2028}') {
        return false;
    }
    if s.starts_with("//") {
        return false;
    }
    match scheme(s) {
        Some(sc) => matches!(sc.as_str(), "http" | "https" | "mailto"),
        None => true,
    }
}

/// Is `raw` a relative address (no scheme, no host), fit for a file of the same site?
#[must_use]
pub fn is_relative(raw: &str) -> bool {
    safe_link(raw) && scheme(raw.trim()).is_none()
}

/// The lower-cased scheme, if the text before the first `:` can be one (it comes before any
/// `/`, `?` or `#`).
fn scheme(s: &str) -> Option<String> {
    let end = s.find([':', '/', '?', '#'])?;
    (s.as_bytes()[end] == b':').then(|| s[..end].to_ascii_lowercase())
}
