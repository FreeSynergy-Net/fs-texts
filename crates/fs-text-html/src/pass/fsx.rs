//! Open a file of the shared folder safely, and copy it byte for byte.
//!
//! Pattern: Facade over the copy sequence (open and check, stream through the probe and the
//! hash, write new). The source must be a plain file reached without any link on the way,
//! with one name only; the output file is created new, never over an existing one.

use super::error::PassError;
use super::media::{MediaType, Prober};
use super::path::PassPath;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

/// Limits of one copy.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Biggest file in bytes.
    pub max_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 10 * 1024 * 1024,
        }
    }
}

/// What was copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copied {
    /// Size in bytes.
    pub bytes: u64,
    /// SHA-256 of the bytes (the same for source and output).
    pub sha256: [u8; 32],
    /// The media type of the extension.
    pub media: MediaType,
}

/// Copy `root/rel` to `dest/rel` unchanged, after all checks.
pub fn copy_file(
    root: &Path,
    rel: &PassPath,
    dest: &Path,
    limits: Limits,
) -> Result<Copied, PassError> {
    let mut src = open_plain(root, rel)?;
    copy_from(&mut src, rel, dest, limits)
}

/// Copy the bytes of `src` to `dest/rel`, after the media and content checks. For a source that
/// was opened by [`open_plain`] (or a store that checked its own file); the output is created
/// new and removed again on a refusal.
pub fn copy_from(
    src: &mut impl Read,
    rel: &PassPath,
    dest: &Path,
    limits: Limits,
) -> Result<Copied, PassError> {
    let media = MediaType::from_extension(&rel.extension().unwrap_or_default())?;
    let to = dest.join(rel.as_str());
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = OpenOptions::new().write(true).create_new(true).open(&to)?;
    let r = stream(src, &mut out, media, limits);
    if r.is_err() {
        drop(out);
        let _ = fs::remove_file(&to);
    }
    r
}

/// Read `src` through the same checks as [`copy_from`] and write nothing: size, SHA-256 and
/// media type (the site builder names media files by their hash before it copies them).
pub fn measure(src: &mut impl Read, rel: &PassPath, limits: Limits) -> Result<Copied, PassError> {
    let media = MediaType::from_extension(&rel.extension().unwrap_or_default())?;
    stream(src, &mut std::io::sink(), media, limits)
}

fn stream(
    src: &mut impl Read,
    out: &mut impl Write,
    media: MediaType,
    limits: Limits,
) -> Result<Copied, PassError> {
    let mut probe = Prober::new(media);
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > limits.max_bytes {
            return Err(PassError::TooBig(limits.max_bytes));
        }
        probe.feed(&buf[..n])?;
        hash.update(&buf[..n]);
        out.write_all(&buf[..n])?;
    }
    probe.finish()?;
    out.flush()?;
    Ok(Copied {
        bytes: total,
        sha256: hash.finalize().into(),
        media,
    })
}

#[cfg(unix)]
/// Open `root/rel` for reading if it is a plain file reached without a link on the way and with
/// one name only.
pub fn open_plain(root: &Path, rel: &PassPath) -> Result<File, PassError> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let mut p = root.to_path_buf();
    let n = rel.segments().count();
    for (i, seg) in rel.segments().enumerate() {
        p.push(seg);
        let m = fs::symlink_metadata(&p)?;
        if m.file_type().is_symlink() {
            return Err(PassError::NotRegular("symbolic link"));
        }
        if i + 1 < n && !m.is_dir() {
            return Err(PassError::NotRegular("a part of the path is not a folder"));
        }
    }
    // Open first, and look at the open descriptor (not at the path again): `O_NOFOLLOW` refuses
    // a link as the last part, `O_NONBLOCK` makes opening a pipe return at once (it is then
    // refused below, it never hangs).
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&p)
        .map_err(|e| {
            if e.raw_os_error() == Some(libc::ELOOP) {
                PassError::NotRegular("symbolic link")
            } else {
                PassError::Io(e)
            }
        })?;
    let m = f.metadata()?;
    if !m.is_file() {
        return Err(PassError::NotRegular("not a regular file"));
    }
    if m.nlink() > 1 {
        return Err(PassError::NotRegular("more than one name (hard link)"));
    }
    // A folder on the way swapped for a link after the walk above: the file we hold must still
    // be the one at `root/rel` with all links resolved.
    let want = fs::canonicalize(root)?.join(rel.as_str());
    if fs::canonicalize(&p)? != want {
        return Err(PassError::NotRegular(
            "a folder on the way changed while opening",
        ));
    }
    Ok(f)
}

#[cfg(not(unix))]
/// Only supported on Unix.
pub fn open_plain(_: &Path, _: &PassPath) -> Result<File, PassError> {
    Err(PassError::NotRegular("only supported on Unix"))
}
