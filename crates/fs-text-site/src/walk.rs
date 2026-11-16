//! Reading a folder: a walk that follows nothing, the order list `.pages`, the address rule.
//!
//! Pattern: none (small pure helpers of the folder adapter).

use crate::error::SiteError;
use std::fs;
use std::path::Path;

const MAX_DEPTH: usize = 12;

pub(crate) fn read_small(p: &Path) -> Result<String, SiteError> {
    let m = fs::symlink_metadata(p)?;
    if !m.is_file() || m.len() > 8 * 1024 {
        return Err(SiteError::Problems(vec![format!(
            "{}: not a plain file of at most 8 KB",
            p.display()
        )]));
    }
    Ok(fs::read_to_string(p)?)
}

/// What one walk found.
#[derive(Default)]
pub(crate) struct Walk {
    pub(crate) files: Vec<String>,
    pub(crate) problems: Vec<String>,
}

/// All plain files below `base`, as `/`-joined relative paths, sorted. Links, devices and the
/// like are problems, never followed.
pub(crate) fn walk(base: &Path, rel: &str, depth: usize, w: &mut Walk) {
    let dir = if rel.is_empty() {
        base.to_path_buf()
    } else {
        base.join(rel)
    };
    let rd = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        // a part that is not there (no `files/`) is no problem; one that cannot be read is
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            w.problems
                .push(format!("{}: cannot be read: {e}", dir.display()));
            return;
        }
    };
    let mut names: Vec<String> = Vec::new();
    for e in rd.flatten() {
        match e.file_name().into_string() {
            Ok(n) => names.push(n),
            Err(n) => w.problems.push(format!(
                "{}: name is not UTF-8: {}",
                dir.display(),
                n.to_string_lossy()
            )),
        }
    }
    names.sort();
    for n in names {
        let r = if rel.is_empty() {
            n.clone()
        } else {
            format!("{rel}/{n}")
        };
        let Ok(m) = fs::symlink_metadata(base.join(&r)) else {
            continue;
        };
        if m.file_type().is_symlink() {
            w.problems.push(format!("{r}: symbolic link"));
        } else if m.is_dir() {
            if depth >= MAX_DEPTH {
                w.problems.push(format!("{r}: too deep"));
            } else {
                walk(base, &r, depth + 1, w);
            }
        } else if m.is_file() {
            w.files.push(r);
        } else {
            w.problems.push(format!("{r}: not a plain file or folder"));
        }
    }
}

pub(crate) fn is_md(f: &str) -> bool {
    Path::new(f).extension().is_some_and(|e| e == "md")
}

pub(crate) fn url_segment_ok(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('.')
        && s.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}

/// The names of a `.pages` file's `nav:` list, in order (`- name`, `- Title: name`; `...` and
/// anything else is ignored).
pub(crate) fn pages_order(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for l in text.lines() {
        let Some(v) = l.trim().strip_prefix("- ") else {
            continue;
        };
        let v = v
            .rsplit(": ")
            .next()
            .unwrap_or(v)
            .trim()
            .trim_matches(['"', '\'']);
        let v = v.strip_suffix(".md").unwrap_or(v).trim_end_matches('/');
        if !v.is_empty() && v != "..." {
            out.push(v.to_string());
        }
    }
    out
}

/// A name that follows the address rule and is close to `s` (`Über uns` gives `ueber-uns`):
/// only a suggestion for the message, nothing is renamed.
pub(crate) fn suggest_segment(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            'ä' | 'Ä' => o.push_str("ae"),
            'ö' | 'Ö' => o.push_str("oe"),
            'ü' | 'Ü' => o.push_str("ue"),
            'ß' => o.push_str("ss"),
            c if c.is_ascii_alphanumeric() => o.push(c.to_ascii_lowercase()),
            '.' | '_' | '-' => o.push(c),
            c if c.is_whitespace() => o.push('-'),
            _ => {}
        }
    }
    let o = o.trim_start_matches('.').to_string();
    if o.is_empty() {
        "page".into()
    } else {
        o
    }
}
