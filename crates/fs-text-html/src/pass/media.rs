//! Media types by extension, and the check of the content against the extension.
//!
//! Pattern: Registry (extension to media type) + State (the [`Prober`] reads a file in chunks).
//! An unknown extension is an error, never `application/octet-stream`. Active types (HTML,
//! script, SVG) are not looked into for scripts (the user's decision), but they are named, so
//! the manifest can mark them.

use super::error::PassError;

/// The media types a user file may have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaType {
    /// `text/html`
    Html,
    /// `text/css`
    Css,
    /// `text/javascript` (`.js`)
    Js,
    /// `text/javascript` (`.mjs`)
    Mjs,
    /// `application/json`
    Json,
    /// `image/svg+xml`
    Svg,
    /// `image/png`
    Png,
    /// `image/jpeg`
    Jpeg,
    /// `image/gif`
    Gif,
    /// `image/webp`
    Webp,
    /// `image/avif`
    Avif,
    /// `image/x-icon`
    Ico,
    /// `font/woff2`
    Woff2,
    /// `text/plain`
    Txt,
    /// `application/pdf`
    Pdf,
    /// `application/xml`
    Xml,
    /// `application/manifest+json`
    Webmanifest,
}

impl MediaType {
    /// The type of a (lower-case) extension.
    pub fn from_extension(ext: &str) -> Result<Self, PassError> {
        Ok(match ext {
            "html" => Self::Html,
            "css" => Self::Css,
            "js" => Self::Js,
            "mjs" => Self::Mjs,
            "json" => Self::Json,
            "svg" => Self::Svg,
            "png" => Self::Png,
            "jpg" | "jpeg" => Self::Jpeg,
            "gif" => Self::Gif,
            "webp" => Self::Webp,
            "avif" => Self::Avif,
            "ico" => Self::Ico,
            "woff2" => Self::Woff2,
            "txt" => Self::Txt,
            "pdf" => Self::Pdf,
            "xml" => Self::Xml,
            "webmanifest" => Self::Webmanifest,
            other => return Err(PassError::UnknownExtension(other.to_string())),
        })
    }

    /// The media type string a server sends.
    #[must_use]
    pub fn mime(self) -> &'static str {
        match self {
            Self::Html => "text/html",
            Self::Css => "text/css",
            Self::Js | Self::Mjs => "text/javascript",
            Self::Json => "application/json",
            Self::Svg => "image/svg+xml",
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Avif => "image/avif",
            Self::Ico => "image/x-icon",
            Self::Woff2 => "font/woff2",
            Self::Txt => "text/plain",
            Self::Pdf => "application/pdf",
            Self::Xml => "application/xml",
            Self::Webmanifest => "application/manifest+json",
        }
    }

    /// Can the browser run something from it (HTML, script, SVG)?
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Html | Self::Js | Self::Mjs | Self::Svg)
    }

    fn is_text(self) -> bool {
        matches!(
            self,
            Self::Html | Self::Css | Self::Js | Self::Mjs | Self::Json | Self::Svg | Self::Txt | Self::Xml | Self::Webmanifest
        )
    }

    fn magic_ok(self, h: &[u8]) -> bool {
        match self {
            Self::Png => h.starts_with(b"\x89PNG\r\n\x1a\n"),
            Self::Jpeg => h.starts_with(&[0xff, 0xd8, 0xff]),
            Self::Gif => h.starts_with(b"GIF87a") || h.starts_with(b"GIF89a"),
            Self::Webp => h.len() >= 12 && &h[..4] == b"RIFF" && &h[8..12] == b"WEBP",
            Self::Avif => h.len() >= 12 && &h[4..8] == b"ftyp" && (&h[8..12] == b"avif" || &h[8..12] == b"avis"),
            Self::Ico => h.starts_with(&[0, 0, 1, 0]),
            Self::Woff2 => h.starts_with(b"wOF2"),
            Self::Pdf => h.starts_with(b"%PDF-"),
            _ => true,
        }
    }

    fn head_ok(self, h: &[u8]) -> Result<(), PassError> {
        let text = String::from_utf8_lossy(h);
        let t = text.trim_start_matches('\u{feff}').trim_start();
        match self {
            Self::Svg if !t.to_ascii_lowercase().contains("<svg") => Err(PassError::ContentMismatch("no <svg> element")),
            Self::Xml if !t.starts_with('<') => Err(PassError::ContentMismatch("XML does not start with `<`")),
            _ => Ok(()),
        }
    }
}

/// Checks the content of one file, chunk by chunk, against its media type.
#[derive(Debug)]
pub struct Prober {
    media: MediaType,
    head: Vec<u8>,
    carry: Vec<u8>,
}

const HEAD: usize = 4096;

impl Prober {
    /// A prober for a file of type `media`.
    #[must_use]
    pub fn new(media: MediaType) -> Self {
        Self { media, head: Vec::new(), carry: Vec::new() }
    }

    /// Look at the next chunk.
    pub fn feed(&mut self, chunk: &[u8]) -> Result<(), PassError> {
        if self.head.len() < HEAD {
            let n = (HEAD - self.head.len()).min(chunk.len());
            self.head.extend_from_slice(&chunk[..n]);
        }
        if !self.media.is_text() {
            return Ok(());
        }
        if chunk.contains(&0) {
            return Err(PassError::ContentMismatch("NUL byte in a text file"));
        }
        let mut buf = std::mem::take(&mut self.carry);
        buf.extend_from_slice(chunk);
        match std::str::from_utf8(&buf) {
            Ok(_) => {}
            Err(e) if e.error_len().is_none() => self.carry = buf[e.valid_up_to()..].to_vec(),
            Err(_) => return Err(PassError::ContentMismatch("not valid UTF-8")),
        }
        Ok(())
    }

    /// The end of the file: the last checks.
    pub fn finish(self) -> Result<(), PassError> {
        if !self.carry.is_empty() {
            return Err(PassError::ContentMismatch("not valid UTF-8"));
        }
        if !self.media.magic_ok(&self.head) {
            return Err(PassError::ContentMismatch("the file does not start like its extension says"));
        }
        self.media.head_ok(&self.head)
    }
}
