//! Where links and images point in the output.
//!
//! Pattern: Strategy — [`LinkResolver`] and [`MediaResolver`] are handed in by the caller (the
//! site builder knows the pretty URLs and the media folder; the plain exporter does not).

use crate::url::{is_relative, safe_link};
use fs_text::LinkValue;

/// The answer of a [`LinkResolver`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkTarget {
    /// Write a link to this address.
    Href(String),
    /// The target does not exist (text without link, and a report entry).
    Missing,
    /// The target exists but is not public (text without link, and a report entry).
    NotPublic,
}

/// Turns the target of a link mark into an address of the output.
pub trait LinkResolver {
    /// Resolve `link` (never called with an address the scheme rule already refused).
    fn resolve(&self, link: &LinkValue) -> LinkTarget;
}

/// The plain resolver: the address as written (a `[[wiki]]` path too); the scheme rule of
/// [`crate::url::safe_link`] still applies.
#[derive(Clone, Copy, Debug, Default)]
pub struct AsWritten;

impl LinkResolver for AsWritten {
    fn resolve(&self, link: &LinkValue) -> LinkTarget {
        LinkTarget::Href(link.path.clone())
    }
}

/// Is this address one the exporter may even ask a resolver about?
#[must_use]
pub fn link_allowed(link: &LinkValue) -> bool {
    safe_link(&link.path)
}

/// An image or other media file as the page uses it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaTarget {
    /// Address in the output.
    pub src: String,
    /// Width in pixels, if known (avoids layout jumps).
    pub width: Option<u32>,
    /// Height in pixels, if known.
    pub height: Option<u32>,
}

/// Turns the target of an image mark into a file of the output.
pub trait MediaResolver {
    /// The output address, or `None` to refuse the image.
    fn resolve(&self, media: &LinkValue) -> Option<MediaTarget>;
}

/// The plain resolver: a relative address as written; anything with a scheme is refused (a
/// page loads nothing from other sites).
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalMedia;

impl MediaResolver for LocalMedia {
    fn resolve(&self, media: &LinkValue) -> Option<MediaTarget> {
        is_relative(&media.path).then(|| MediaTarget {
            src: media.path.trim().to_string(),
            width: None,
            height: None,
        })
    }
}
