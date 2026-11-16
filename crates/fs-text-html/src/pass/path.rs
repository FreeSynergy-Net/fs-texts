//! The path of a passed-through file.
//!
//! Pattern: Newtype / Value object — valid on construction, so no `PassPath` can leave its
//! folder: relative, no `..`, ASCII `[A-Za-z0-9._-]` (so also NFC), no hidden parts.

use super::error::PassError;

const MAX_BYTES: usize = 240;
const MAX_DEPTH: usize = 12;
const MAX_SEGMENT: usize = 100;

/// A checked relative path with `/` between segments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PassPath(String);

impl PassPath {
    /// Check `s`. Refused: empty, leading `/`, `\`, `.`/`..`, empty segments, characters outside
    /// `[A-Za-z0-9._-]`, hidden segments (a leading `.`) except a top `.well-known`, more than 240
    /// bytes, 12 segments or 100 bytes in one segment.
    pub fn parse(s: &str) -> Result<Self, PassError> {
        if s.is_empty() {
            return Err(PassError::EmptyPath);
        }
        if s.starts_with('/') {
            return Err(PassError::Absolute);
        }
        if s.len() > MAX_BYTES {
            return Err(PassError::TooLong);
        }
        if let Some(c) = s
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || "._-/".contains(*c)))
        {
            return Err(PassError::BadChar(c));
        }
        let segs: Vec<&str> = s.split('/').collect();
        if segs.len() > MAX_DEPTH {
            return Err(PassError::TooLong);
        }
        for (i, seg) in segs.iter().enumerate() {
            if seg.is_empty() {
                return Err(PassError::EmptySegment);
            }
            if *seg == "." || *seg == ".." {
                return Err(PassError::DotSegment);
            }
            if seg.len() > MAX_SEGMENT {
                return Err(PassError::TooLong);
            }
            if seg.starts_with('.') && !(i == 0 && *seg == ".well-known") {
                return Err(PassError::Hidden);
            }
        }
        Ok(Self(s.to_string()))
    }

    /// The path as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The segments.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    /// The lower-cased extension of the last segment (without the dot).
    #[must_use]
    pub fn extension(&self) -> Option<String> {
        let last = self.0.rsplit('/').next()?;
        let (stem, ext) = last.rsplit_once('.')?;
        (!stem.is_empty()).then(|| ext.to_ascii_lowercase())
    }

    /// Does the path lie under `prefix` (a folder name like `de` or `_fs`, or a file name)?
    /// Compared by whole segments and without regard to letter case.
    #[must_use]
    pub fn is_under(&self, prefix: &str) -> bool {
        self.segments()
            .next()
            .is_some_and(|first| first.eq_ignore_ascii_case(prefix.trim_matches('/')))
    }
}
