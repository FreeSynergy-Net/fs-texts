//! Why a file is not passed through.
//!
//! Pattern: none (error list).

use std::fmt;

/// A refusal. Every variant stops the build; the caller collects all of them.
#[derive(Debug)]
pub enum PassError {
    /// The path is empty.
    EmptyPath,
    /// The path starts with `/`.
    Absolute,
    /// The path has a `.` or `..` segment.
    DotSegment,
    /// The path has an empty segment (`a//b`) or ends with `/`.
    EmptySegment,
    /// A character outside `[A-Za-z0-9._-]` (and `/` between segments), `\` included.
    BadChar(char),
    /// A hidden file or folder (other than `.well-known` at the top).
    Hidden,
    /// Too long or too deep.
    TooLong,
    /// Same path twice, or two paths that differ only in letter case.
    Duplicate(String),
    /// A file and a folder with the same path.
    FileAndFolder(String),
    /// The path lies under a prefix the site generates itself.
    Reserved(String),
    /// No known media type for the extension (never `application/octet-stream`).
    UnknownExtension(String),
    /// The content does not fit the extension.
    ContentMismatch(&'static str),
    /// Bigger than the limit.
    TooBig(u64),
    /// A link, device, pipe, folder, or a file with more than one name.
    NotRegular(&'static str),
    /// The file system said no.
    Io(std::io::Error),
}

impl fmt::Display for PassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath => write!(f, "empty path"),
            Self::Absolute => write!(f, "absolute path"),
            Self::DotSegment => write!(f, "`.` or `..` in path"),
            Self::EmptySegment => write!(f, "empty path segment"),
            Self::BadChar(c) => write!(f, "character {c:?} not allowed in a path"),
            Self::Hidden => write!(f, "hidden file or folder"),
            Self::TooLong => write!(f, "path too long or too deep"),
            Self::Duplicate(p) => write!(f, "path twice (or only the case differs): {p}"),
            Self::FileAndFolder(p) => write!(f, "file and folder at once: {p}"),
            Self::Reserved(p) => write!(f, "path under the generated prefix {p}"),
            Self::UnknownExtension(e) => write!(f, "no media type for the extension {e:?}"),
            Self::ContentMismatch(m) => write!(f, "content does not fit the extension: {m}"),
            Self::TooBig(n) => write!(f, "file bigger than the limit of {n} bytes"),
            Self::NotRegular(w) => write!(f, "not a plain file: {w}"),
            Self::Io(e) => write!(f, "file system: {e}"),
        }
    }
}

impl std::error::Error for PassError {}

impl From<std::io::Error> for PassError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
