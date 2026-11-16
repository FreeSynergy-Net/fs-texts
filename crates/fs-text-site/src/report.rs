//! The result of a build: what was found, what stopped it.
//!
//! Pattern: Value object. Gates (`Gate::Fail`) turn a finding into a stop; the policy is a
//! setting of the caller (the release workflow), not a rule of the builder.

use std::fmt::Write;

/// What a kind of finding does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    /// Report only.
    Warn,
    /// Stop the build.
    Fail,
}

/// The gates of one build.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    /// A link to a page or file that does not exist (or is not public).
    pub dead_links: Gate,
    /// An image without alt text.
    pub missing_alt: Gate,
    /// An image whose file is missing.
    pub missing_media: Gate,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            dead_links: Gate::Fail,
            missing_alt: Gate::Warn,
            missing_media: Gate::Fail,
        }
    }
}

/// One finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Where (a page, a file, `site.toml`).
    pub place: String,
    /// What.
    pub message: String,
    /// Does it stop the build?
    pub fatal: bool,
}

/// What a build says about itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BuildReport {
    /// The findings, in the order of the build.
    pub findings: Vec<Finding>,
    /// Pages written (generated contents pages included).
    pub pages: usize,
    /// Files in the manifest.
    pub files: usize,
    /// Links to a page, file or anchor of the site that does not exist (counted whatever the gate).
    pub dead_links: usize,
    /// SHA-256 of `site-manifest.json` (the manifest names every other file by hash).
    pub manifest_sha256: String,
}

impl BuildReport {
    /// Add a finding.
    pub fn note(&mut self, place: &str, message: impl Into<String>, fatal: bool) {
        self.findings.push(Finding {
            place: place.to_string(),
            message: message.into(),
            fatal,
        });
    }

    /// A link to nothing: counted, and a stop if the gate says so.
    pub fn dead_link(&mut self, place: &str, message: impl Into<String>, gate: Gate) {
        self.dead_links += 1;
        self.note(place, message, gate == Gate::Fail);
    }

    /// Is there a finding that stops the build?
    #[must_use]
    pub fn has_fatal(&self) -> bool {
        self.findings.iter().any(|f| f.fatal)
    }

    /// The report as text, one line per finding.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut o = String::new();
        for f in &self.findings {
            let _ = writeln!(
                o,
                "{}: {}: {}",
                if f.fatal { "error" } else { "warning" },
                f.place,
                f.message
            );
        }
        let _ = writeln!(
            o,
            "{} pages, {} files, {} dead links, manifest sha256 {}",
            self.pages, self.files, self.dead_links, self.manifest_sha256
        );
        o
    }
}
