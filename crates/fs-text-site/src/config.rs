//! `site.toml`: reading and checking schema 1 (fs-release stage 0) and schema 2 (several sites
//! on one domain). Specification: `/srv/FreeSynergy/.scratch/site-toml-schema-v2.md`.
//!
//! Pattern: Builder from a raw, typed form (serde, unknown keys refused) into a checked value
//! object. [`SiteConfig`] is valid on construction; nothing outside re-checks a name.
//! `names` and `dns.records` stay the only thing a later DNS adapter needs to read.

use crate::error::ConfigError;
use crate::host::{Host, PathPrefix};
pub use crate::lang::Lang;
use crate::raw::{build_rules, csp, dns_records, Raw, RawSite};
pub use crate::rules::{BuildRules, DeadLinks};

const MAX_BYTES: usize = 8 * 1024;
const MAX_NAMES: usize = 10;
const MAX_LANGUAGES: usize = 8;

/// A record type a site wants in DNS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DnsRecord {
    /// IPv4 address.
    A,
    /// IPv6 address.
    Aaaa,
    /// Certification authority authorisation.
    Caa,
}

impl DnsRecord {
    fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::Aaaa => "AAAA",
            Self::Caa => "CAA",
        }
    }
}

/// `csp.mode`: what the files of the site may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CspMode {
    /// No script anywhere: strict.
    Static,
    /// The site passes through files of the user that may carry their own script.
    Passthrough,
}

/// `[[csp.area]]`: the rule of one folder of the site (and everything below it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CspArea {
    /// The folder, below the site's own prefix (`/apps`).
    pub path: PathPrefix,
    /// What files there may do.
    pub mode: CspMode,
    /// A complete policy for the generated pages of this folder (`<lang>/<folder>/...`).
    pub policy: Option<String>,
    /// The policy may allow more than the site's (`allow_loosen = true`); the build warns.
    pub allow_loosen: bool,
}

/// A checked site description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteConfig {
    /// Schema the file was written in (1 or 2).
    pub schema: u8,
    /// Short name of the site; names its output folder.
    pub id: String,
    /// Name shown in the page frame.
    pub name: String,
    /// The host used in absolute addresses (canonical, `hreflang`, sitemap): `names.primary`,
    /// the one place the name is written.
    pub host: Host,
    /// The host names the site answers to (primary first, then aliases).
    pub names: Vec<Host>,
    /// Where on the host the site lives.
    pub path_prefix: PathPrefix,
    /// Languages of the site; the folder names under the root.
    pub languages: Vec<Lang>,
    /// The language of a fallback and of `x-default`.
    pub default_language: Lang,
    /// Wanted DNS record types.
    pub dns_records: Vec<DnsRecord>,
    /// `csp.mode`.
    pub csp_mode: CspMode,
    /// `csp.policy`, replaces the preset of the mode.
    pub csp_policy: Option<String>,
    /// Rules of single folders; the longest matching folder wins, else `csp_mode`.
    pub csp_areas: Vec<CspArea>,
    /// `[build]`.
    pub build: BuildRules,
}

impl SiteConfig {
    /// Read and check a `site.toml` (schema 1 or 2).
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        if text.len() > MAX_BYTES {
            return Err(ConfigError::TooBig);
        }
        let table: toml::Table = text.parse().map_err(|e: toml::de::Error| {
            ConfigError::Toml(crate::host::printable(&e.message().replace('\n', " ")))
        })?;
        let schema = match table.get("schema") {
            Some(toml::Value::Integer(n @ (1 | 2))) => *n,
            Some(toml::Value::Integer(n)) => return Err(ConfigError::UnknownSchema(*n)),
            _ => {
                return Err(ConfigError::Toml(
                    "`schema` must be the number 1 or 2".into(),
                ))
            }
        };
        let raw: Raw = toml::from_str(text).map_err(|e| {
            ConfigError::Toml(crate::host::printable(&e.message().replace('\n', " ")))
        })?;
        Self::from_raw(raw, schema)
    }

    fn from_raw(raw: Raw, schema: i64) -> Result<Self, ConfigError> {
        if raw.purpose != "website" {
            return Err(ConfigError::BadPurpose);
        }
        let v1 = schema == 1;
        match (&raw.site, v1) {
            (Some(_), true) => {
                return Err(ConfigError::SiteTable("a [site] table needs schema = 2"))
            }
            (None, false) => return Err(ConfigError::SiteTable("schema 2 needs a [site] table")),
            _ => {}
        }
        let host_of = |s: &str| {
            if v1 {
                Host::parse_v1(s)
            } else {
                Host::parse(s)
            }
        };
        let primary = host_of(&raw.names.primary)?;
        let mut names = vec![primary.clone()];
        let aliases = raw.names.aliases.unwrap_or_default();
        if aliases.len() > MAX_NAMES - 1 {
            return Err(ConfigError::Names("more than 9 aliases".into()));
        }
        for a in &aliases {
            let h = host_of(a)?;
            if names.contains(&h) {
                return Err(ConfigError::Names(format!("{} is there twice", h.as_str())));
            }
            names.push(h);
        }
        let dns_records = dns_records(raw.dns)?;
        if raw.acme.method != "http-01" {
            return Err(ConfigError::AcmeMethod(if raw.acme.method == "dns-01" {
                "\"dns-01\" is reserved and not implemented yet; use \"http-01\"".into()
            } else {
                "only \"http-01\" is allowed".into()
            }));
        }
        let (csp_mode, csp_policy, csp_areas) = csp(raw.csp, v1)?;
        let mut cfg = Self {
            schema: if v1 { 1 } else { 2 },
            id: "main".into(),
            name: primary.as_str().to_string(),
            host: primary,
            names,
            path_prefix: PathPrefix::root(),
            languages: vec![Lang::parse("en")?],
            default_language: Lang::parse("en")?,
            dns_records,
            csp_mode,
            csp_policy,
            csp_areas,
            build: build_rules(raw.build, v1)?,
        };
        if let Some(s) = raw.site {
            cfg.apply_site(s)?;
        }
        Ok(cfg)
    }

    fn apply_site(&mut self, s: RawSite) -> Result<(), ConfigError> {
        let id_ok = !s.id.is_empty()
            && s.id.len() <= 32
            && s.id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !s.id.starts_with('-')
            && !s.id.ends_with('-');
        if !id_ok {
            return Err(ConfigError::BadId(
                "1 to 32 characters of a-z 0-9 -, not starting or ending with -".into(),
            ));
        }
        self.id = s.id;
        if let Some(n) = s.name {
            if n.is_empty() || n.chars().count() > 80 || n.chars().any(char::is_control) {
                return Err(ConfigError::BadName);
            }
            self.name = n;
        } else {
            self.name.clone_from(&self.id);
        }
        if let Some(p) = s.path_prefix {
            self.path_prefix = PathPrefix::parse(&p)?;
        }
        if let Some(l) = s.languages {
            if l.is_empty() || l.len() > MAX_LANGUAGES {
                return Err(ConfigError::BadLanguage("1 to 8 languages".into()));
            }
            let mut out: Vec<Lang> = Vec::new();
            for x in &l {
                let x = Lang::parse(x)?;
                if out.contains(&x) {
                    return Err(ConfigError::BadLanguage(format!(
                        "{} is there twice",
                        x.as_str()
                    )));
                }
                out.push(x);
            }
            self.languages = out;
            self.default_language = self.languages[0].clone();
        }
        if let Some(d) = s.default_language {
            let d = Lang::parse(&d)?;
            if !self.languages.contains(&d) {
                return Err(ConfigError::DefaultNotListed(d.as_str().to_string()));
            }
            self.default_language = d;
        }
        Ok(())
    }

    /// The `site.toml` of the output folder: schema 1, which the release check of fs-release
    /// reads. Only what that file knows (`names`, `dns`, `acme`, `csp`); the schema 2 fields
    /// (`site`) stay in the source. Values are checked, so nothing needs escaping.
    #[must_use]
    pub fn to_schema1(&self) -> String {
        let q = |s: &str| format!("\"{s}\"");
        let mut o = String::from("schema = 1\npurpose = \"website\"\n\n[names]\n");
        o.push_str(&format!("primary = {}\n", q(self.names[0].as_str())));
        if self.names.len() > 1 {
            let a: Vec<String> = self.names[1..].iter().map(|n| q(n.as_str())).collect();
            o.push_str(&format!("aliases = [{}]\n", a.join(", ")));
        }
        let r: Vec<String> = self.dns_records.iter().map(|r| q(r.name())).collect();
        o.push_str(&format!("\n[dns]\nrecords = [{}]\n", r.join(", ")));
        o.push_str("\n[acme]\nmethod = \"http-01\"\n\n[csp]\n");
        // schema 1 has one mode per site: passthrough as soon as any folder needs it
        let any = self.csp_mode == CspMode::Passthrough
            || self
                .csp_areas
                .iter()
                .any(|a| a.mode == CspMode::Passthrough);
        let m = if any { "passthrough" } else { "static" };
        o.push_str(&format!("mode = {}\n", q(m)));
        if let Some(p) = &self.csp_policy {
            o.push_str(&format!("policy = {}\n", q(p)));
        }
        o
    }

    /// The prefix as it starts every address of the site (`""` for the root).
    #[must_use]
    pub fn url_prefix(&self) -> String {
        self.path_prefix.as_url()
    }

    /// `https://host/prefix`, without a trailing slash.
    #[must_use]
    pub fn origin(&self) -> String {
        format!("https://{}{}", self.host.as_str(), self.url_prefix())
    }
}

impl SiteConfig {
    /// What files at `rel` (a path below the site, no leading `/`) may do: the longest matching
    /// `[[csp.area]]`, else `csp.mode`.
    #[must_use]
    pub fn csp_mode_for(&self, rel: &str) -> CspMode {
        let segs: Vec<String> = rel.split('/').map(str::to_string).collect();
        self.csp_areas
            .iter()
            .filter(|a| a.path.segments().len() < segs.len() && segs.starts_with(a.path.segments()))
            .max_by_key(|a| a.path.segments().len())
            .map_or(self.csp_mode, |a| a.mode)
    }
}
