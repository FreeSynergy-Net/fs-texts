//! Host names and path prefixes of a site.
//!
//! Pattern: Newtype / Value object — a [`Host`] or a [`PathPrefix`] is valid on construction, so
//! no later code (the Zentinel template, the DNS adapter) ever sees a name with a quote, a
//! line break, a brace or a path with `..` in it.

use crate::error::ConfigError;

/// A lower-case ASCII DNS name (labels of `a-z 0-9 -`; an international name only as its A-label
/// `xn--…`, in the last label too). Never a wildcard, never with a trailing dot.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Host(String);

const MAX_HOST: usize = 253;

impl Host {
    /// Check `s` against the rules of schema 2.
    pub fn parse(s: &str) -> Result<Self, ConfigError> {
        Self::parse_with(s, true)
    }

    /// Check `s` against the (narrower) rules of schema 1: the last label only letters.
    pub fn parse_v1(s: &str) -> Result<Self, ConfigError> {
        Self::parse_with(s, false)
    }

    fn parse_with(s: &str, a_label_tld: bool) -> Result<Self, ConfigError> {
        let bad = |why: &'static str| ConfigError::BadHost {
            name: printable(s),
            why,
        };
        if s.is_empty() {
            return Err(bad("empty"));
        }
        if !s.is_ascii() {
            return Err(bad("not ASCII (give an international name as its xn-- form)"));
        }
        if s.len() > MAX_HOST {
            return Err(bad("longer than 253 bytes"));
        }
        if s.ends_with('.') {
            return Err(bad("trailing dot"));
        }
        let labels: Vec<&str> = s.split('.').collect();
        if labels.len() < 2 {
            return Err(bad("needs at least two labels"));
        }
        for (i, l) in labels.iter().enumerate() {
            let last = i + 1 == labels.len();
            if l.is_empty() {
                return Err(bad("empty label"));
            }
            if l.len() > 63 {
                return Err(bad("label longer than 63 bytes"));
            }
            if !l
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                return Err(bad(
                    "only lower-case letters, digits and `-` (no wildcard, no upper case)",
                ));
            }
            if l.starts_with('-') || l.ends_with('-') {
                return Err(bad("label starts or ends with `-`"));
            }
            if l.starts_with("xn--") && l.len() < 5 {
                return Err(bad("empty xn-- label"));
            }
            if last {
                let alpha = l.len() >= 2 && l.bytes().all(|b| b.is_ascii_lowercase());
                let a_label = a_label_tld && l.starts_with("xn--");
                if !alpha && !a_label {
                    return Err(bad("the last label must be 2 to 63 letters (or an xn-- label)"));
                }
            }
        }
        Ok(Self(s.to_string()))
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `s` with every control character and quote shown as an escape, for messages (a refused name
/// may contain anything, also a line break that would forge a second log line).
#[must_use]
pub fn printable(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars().take(80) {
        if c.is_control() {
            o.push_str(&c.escape_default().to_string());
        } else {
            o.push(c);
        }
    }
    o
}

/// The path under which a site lives on its host: empty for the root, else `/a/b` (no trailing
/// slash). Segments `a-z 0-9 . _ -`, none hidden, at most 4, `_fs` never the first.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathPrefix(Vec<String>);

impl PathPrefix {
    /// The root (`/`).
    #[must_use]
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Check `s`: `/` or `/seg(/seg)*`.
    pub fn parse(s: &str) -> Result<Self, ConfigError> {
        let bad = |why: &'static str| ConfigError::BadPrefix {
            prefix: printable(s),
            why,
        };
        if s == "/" {
            return Ok(Self::root());
        }
        let Some(rest) = s.strip_prefix('/') else {
            return Err(bad("must start with `/`"));
        };
        if s.len() > 100 {
            return Err(bad("longer than 100 bytes"));
        }
        let mut segs = Vec::new();
        for seg in rest.split('/') {
            if seg.is_empty() {
                return Err(bad("empty segment (`//` or a trailing `/`)"));
            }
            if seg == "." || seg == ".." {
                return Err(bad("`.` or `..` segment"));
            }
            if seg.starts_with('.') {
                return Err(bad("hidden segment"));
            }
            if !seg.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            }) {
                return Err(bad("only a-z 0-9 . _ - in a segment (no `%`, `\\`, `?`, `#`)"));
            }
            segs.push(seg.to_string());
        }
        if segs.len() > 4 {
            return Err(bad("more than 4 segments"));
        }
        if segs[0] == "_fs" {
            return Err(bad("`_fs` is reserved"));
        }
        Ok(Self(segs))
    }

    /// The segments.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    /// Is this the root?
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// `""` for the root, else `/a/b`.
    #[must_use]
    pub fn as_url(&self) -> String {
        self.0.iter().fold(String::new(), |mut o, s| {
            o.push('/');
            o.push_str(s);
            o
        })
    }

    /// Is `self` a part of `other`, or equal (segment by segment)?
    #[must_use]
    pub fn is_prefix_of(&self, other: &Self) -> bool {
        other.0.len() >= self.0.len() && other.0[..self.0.len()] == self.0[..]
    }
}
