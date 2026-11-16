//! The scanner of raw HTML tags and character references.
//!
//! Pattern: none (pure functions). A tag that is not well formed is not a tag: the `<` stays text.

use crate::escape;

pub(crate) struct Tag {
    pub len: usize,
    pub closing: bool,
    pub name: String,
    pub attrs: Vec<(String, String)>,
}

/// Length of the character reference at the start of `s` (`&amp;`, `&#65;`, `&#x41;`), else 0.
pub(crate) fn entity_len(s: &str) -> usize {
    let b = s.as_bytes();
    let Some(semi) = s.find(';').filter(|p| *p <= 33) else {
        return 0;
    };
    let body = &b[1..semi];
    let ok = match body {
        [b'#', b'x' | b'X', hex @ ..] => {
            (1..=6).contains(&hex.len()) && hex.iter().all(u8::is_ascii_hexdigit)
        }
        [b'#', dec @ ..] => (1..=7).contains(&dec.len()) && dec.iter().all(u8::is_ascii_digit),
        [first, rest @ ..] => {
            first.is_ascii_alphabetic()
                && !rest.is_empty()
                && rest.iter().all(u8::is_ascii_alphanumeric)
        }
        [] => false,
    };
    if ok {
        semi + 1
    } else {
        0
    }
}

/// An attribute value with its character references kept (`&amp;` stays), quotes escaped.
pub(crate) fn keep_entities(v: &str, out: &mut String) {
    let mut i = 0;
    while i < v.len() {
        let rest = &v[i..];
        let n = if rest.starts_with('&') {
            entity_len(rest)
        } else {
            0
        };
        if n > 0 {
            out.push_str(&rest[..n]);
            i += n;
        } else {
            let c = rest.chars().next().unwrap_or(' ');
            escape::attr(c.encode_utf8(&mut [0; 4]), out);
            i += c.len_utf8();
        }
    }
}

pub(crate) fn parse_tag(s: &str) -> Option<Tag> {
    let b = s.as_bytes();
    let mut p = 1;
    let closing = b.get(p) == Some(&b'/');
    if closing {
        p += 1;
    }
    let start = p;
    if !b.get(p)?.is_ascii_alphabetic() {
        return None;
    }
    while b
        .get(p)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'-')
    {
        p += 1;
    }
    let name = s[start..p].to_ascii_lowercase();
    let mut attrs = Vec::new();
    loop {
        while b.get(p).is_some_and(u8::is_ascii_whitespace) {
            p += 1;
        }
        match b.get(p)? {
            b'>' => {
                p += 1;
                break;
            }
            b'/' if b.get(p + 1) == Some(&b'>') => {
                p += 2;
                break;
            }
            c if c.is_ascii_alphabetic() || *c == b'_' || *c == b':' => {
                let a = p;
                while b
                    .get(p)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || b"_:.-".contains(c))
                {
                    p += 1;
                }
                let key = s[a..p].to_ascii_lowercase();
                while b.get(p).is_some_and(u8::is_ascii_whitespace) {
                    p += 1;
                }
                let mut val = String::new();
                if b.get(p) == Some(&b'=') {
                    p += 1;
                    while b.get(p).is_some_and(u8::is_ascii_whitespace) {
                        p += 1;
                    }
                    (val, p) = attr_value(s, p)?;
                }
                attrs.push((key, val));
            }
            _ => return None,
        }
    }
    Some(Tag {
        len: p,
        closing,
        name,
        attrs,
    })
}

/// The value after `=` at `p`: quoted, or a run without space, quote, `=`, `<`, `>`, backtick.
fn attr_value(s: &str, mut p: usize) -> Option<(String, usize)> {
    let b = s.as_bytes();
    if let q @ (b'"' | b'\'') = b.get(p)? {
        let end = s[p + 1..].find(*q as char)? + p + 1;
        return Some((s[p + 1..end].to_string(), end + 1));
    }
    let a = p;
    while b
        .get(p)
        .is_some_and(|c| !c.is_ascii_whitespace() && !b"\"'=<>`".contains(c))
    {
        p += 1;
    }
    (p > a).then(|| (s[a..p].to_string(), p))
}
