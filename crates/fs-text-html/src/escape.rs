//! Escaping of text and attribute values for HTML.
//!
//! Pattern: none (pure functions).

/// Append `s` as HTML text (`&`, `<`, `>` escaped, NUL replaced).
pub fn text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\0' => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
}

/// Append `s` as a double-quoted attribute value (without the quotes).
pub fn attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\0' => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
}

/// `s` escaped as text, as a new string.
#[must_use]
pub fn text_string(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    text(s, &mut o);
    o
}

/// `s` escaped as attribute value, as a new string.
#[must_use]
pub fn attr_string(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    attr(s, &mut o);
    o
}

/// Keep only `[a-z0-9-]` of `s` (lower-cased), for use inside a class name.
#[must_use]
pub fn class_part(s: &str) -> String {
    s.chars()
        .filter_map(|c| match c {
            'a'..='z' | '0'..='9' | '-' => Some(c),
            'A'..='Z' => Some(c.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}
