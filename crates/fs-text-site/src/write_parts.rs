//! The first steps of phase 2: user files, style sheet, media.
//!
//! Pattern: the same Builder steps as `write`, in a second file (one context, several parts).

use crate::error::SiteError;
use crate::index::media_url;
use crate::outfs::{hex, Entry, Kind};
use crate::report::BuildReport;
use crate::source::FileEntry;
use crate::write::{problem, Writer};
use fs_text_html::pass::{copy_from, PassPath};
use fs_text_html::theme;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

impl Writer<'_> {
    /// The user's files, byte for byte; the hash must equal the one measured before.
    pub(crate) fn copy_pass(
        &mut self,
        passed: &[(FileEntry, [u8; 32], bool)],
    ) -> Result<(), SiteError> {
        for (f, sha, active) in passed {
            let mut r = self.src.open(f).map_err(problem)?;
            let c = copy_from(&mut r, &f.path, self.out.root(), self.limits)
                .map_err(|e| problem(format!("{}: {e}", f.path.as_str())))?;
            if c.sha256 != *sha {
                return Err(problem(format!(
                    "{}: changed while the build ran",
                    f.path.as_str()
                )));
            }
            self.supplied.insert(f.path.as_str().to_string());
            self.out.record(Entry {
                path: f.path.as_str().to_string(),
                sha256: c.sha256,
                bytes: c.bytes,
                media: c.media.mime().to_string(),
                kind: Kind::Passthrough,
                active: *active,
            });
        }
        Ok(())
    }

    pub(crate) fn style_sheet(&mut self) -> Result<(), SiteError> {
        let name = format!(
            "_fs/theme.{}.css",
            &hex(&Sha256::digest(theme::CSS.as_bytes()))[..16]
        );
        self.out
            .put(&name, theme::CSS.as_bytes(), "text/css", Kind::Generated)?;
        self.css_url = format!("{}/{name}", self.prefix);
        Ok(())
    }

    /// The media files pages use (named by hash); the unused ones are reported, not published.
    pub(crate) fn copy_media(&mut self, rep: &mut BuildReport) -> Result<(), SiteError> {
        let mut used: BTreeSet<String> = BTreeSet::new();
        let media_prefix = media_url(&self.prefix, "");
        for r in self.rendered {
            for a in r.html.links.iter().chain(&r.html.media) {
                if let Some(n) = a.strip_prefix(&media_prefix) {
                    used.insert(n.split('#').next().unwrap_or(n).to_string());
                }
            }
        }
        let mut written: BTreeSet<String> = BTreeSet::new();
        for m in self.index.media.values() {
            if !used.contains(&m.name) {
                rep.note(
                    m.entry.path.as_str(),
                    "media file no page uses (not published)",
                    false,
                );
            } else if written.insert(m.name.clone()) {
                let rel = PassPath::parse(&format!("_fs/media/{}", m.name)).map_err(problem)?;
                let mut rd = self.src.open(&m.entry).map_err(problem)?;
                let c = copy_from(&mut rd, &rel, self.out.root(), self.limits)
                    .map_err(|e| problem(format!("{}: {e}", rel.as_str())))?;
                if c.sha256 != m.sha256 {
                    return Err(problem(format!(
                        "{}: changed while the build ran",
                        rel.as_str()
                    )));
                }
                self.out.record(Entry {
                    path: rel.as_str().to_string(),
                    sha256: c.sha256,
                    bytes: c.bytes,
                    media: c.media.mime().to_string(),
                    kind: Kind::Media,
                    active: false,
                });
            }
        }
        Ok(())
    }
}
