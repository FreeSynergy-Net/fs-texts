//! The `site.toml` of the output folder (the package root), as the release check of fs-release
//! reads it.
//!
//! Pattern: Template method (a fixed text with the checked values filled in). A site made from a
//! schema 2 source is written in schema 2: `id`, `path_prefix`, the languages and the folder
//! rules (`[[csp.area]]`, each with its `mode`) must reach the server, or two sites of one
//! domain cannot be told apart. Only keys that fs-release knows are written: `[build]` and
//! `allow_loosen` are the business of the source, not of the package. A schema 1 source
//! (one site at the root of its host) stays schema 1, which fs-release still reads unchanged.

use crate::config::{CspMode, SiteConfig};

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn list<'a>(items: impl Iterator<Item = &'a str>) -> String {
    format!("[{}]", items.map(quote).collect::<Vec<_>>().join(", "))
}

fn mode_word(m: CspMode) -> &'static str {
    match m {
        CspMode::Static => "static",
        CspMode::Passthrough => "passthrough",
    }
}

impl SiteConfig {
    /// The `site.toml` written into the output folder; read it back with [`SiteConfig::parse`].
    /// The folder of the output is named `id` (see [`crate::SiteSet::out_name`]).
    #[must_use]
    pub fn to_package_toml(&self) -> String {
        let v2 = self.schema == 2;
        let mut o = format!("schema = {}\npurpose = \"website\"\n", self.schema);
        if v2 {
            o.push_str(&format!("\n[site]\nid = {}\n", quote(&self.id)));
            o.push_str(&format!("name = {}\n", quote(&self.name)));
            let p = if self.path_prefix.is_root() {
                "/".to_string()
            } else {
                self.url_prefix()
            };
            o.push_str(&format!("path_prefix = {}\n", quote(&p)));
            o.push_str(&format!(
                "languages = {}\n",
                list(self.languages.iter().map(crate::Lang::as_str))
            ));
            o.push_str(&format!(
                "default_language = {}\n",
                quote(self.default_language.as_str())
            ));
        }
        o.push_str(&format!(
            "\n[names]\nprimary = {}\n",
            quote(self.names[0].as_str())
        ));
        if self.names.len() > 1 {
            o.push_str(&format!(
                "aliases = {}\n",
                list(self.names[1..].iter().map(crate::Host::as_str))
            ));
        }
        o.push_str(&format!(
            "\n[dns]\nrecords = {}\n",
            list(self.dns_records.iter().map(|r| r.name()))
        ));
        o.push_str("\n[acme]\nmethod = \"http-01\"\n\n[csp]\n");
        o.push_str(&format!("mode = {}\n", quote(mode_word(self.csp_mode))));
        if let Some(p) = &self.csp_policy {
            o.push_str(&format!("policy = {}\n", quote(p)));
        }
        for a in self.csp_areas.iter().filter(|_| v2) {
            o.push_str(&format!(
                "\n[[csp.area]]\npath = {}\nmode = {}\n",
                quote(&a.path.as_url()),
                quote(mode_word(a.mode))
            ));
            if let Some(p) = &a.policy {
                o.push_str(&format!("policy = {}\n", quote(p)));
            }
        }
        o
    }
}
