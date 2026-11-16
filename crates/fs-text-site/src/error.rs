//! What can go wrong, as plain lists.
//!
//! Pattern: none (error enums). A build collects *all* problems before it stops, so one run
//! shows everything that must be fixed.

use std::fmt;

/// A refusal of `site.toml` or of the set of sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// More than 8 KB.
    TooBig,
    /// Not valid TOML, or an unknown key (the message of the parser).
    Toml(String),
    /// `schema` is neither 1 nor 2.
    UnknownSchema(i64),
    /// `purpose` is not `website`.
    BadPurpose,
    /// A `[site]` table in a schema 1 file, or none in a schema 2 file.
    SiteTable(&'static str),
    /// A host name that breaks the rules.
    BadHost {
        /// The name (made printable).
        name: String,
        /// The rule.
        why: &'static str,
    },
    /// A path prefix that breaks the rules.
    BadPrefix {
        /// The prefix (made printable).
        prefix: String,
        /// The rule.
        why: &'static str,
    },
    /// A name twice, an alias equal to the primary name, or too many names.
    Names(String),
    /// `site.id` breaks the rule.
    BadId(String),
    /// `site.name` breaks the rule.
    BadName,
    /// A language tag breaks the rule, is twice, or too many.
    BadLanguage(String),
    /// `default_language` is not in `languages`.
    DefaultNotListed(String),
    /// `dns.records` breaks the rule.
    DnsRecords(String),
    /// `acme.method` breaks the rule.
    AcmeMethod(String),
    /// `csp.mode` breaks the rule.
    CspMode(String),
    /// `csp.policy` breaks the rule.
    CspPolicy,
    /// Two sites claim the same host and path.
    SiteCollision {
        /// First site.
        a: String,
        /// Second site.
        b: String,
        /// The shared name.
        name: String,
        /// The shared path (`/` for the root).
        path: String,
    },
    /// A site below another one would sit on a name the outer site generates.
    NestedShadowed {
        /// The inner site.
        inner: String,
        /// The outer site.
        outer: String,
        /// The first segment of the inner prefix, below the outer prefix.
        segment: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooBig => write!(f, "site.toml is bigger than 8 KB"),
            Self::Toml(m) => write!(f, "site.toml: {m}"),
            Self::UnknownSchema(n) => write!(f, "schema {n} is not known (1 or 2)"),
            Self::BadPurpose => write!(f, "purpose must be \"website\""),
            Self::SiteTable(m) => write!(f, "[site]: {m}"),
            Self::BadHost { name, why } => write!(f, "host name {name:?}: {why}"),
            Self::BadPrefix { prefix, why } => write!(f, "path prefix {prefix:?}: {why}"),
            Self::Names(m) => write!(f, "names: {m}"),
            Self::BadId(m) => write!(f, "site.id: {m}"),
            Self::BadName => write!(f, "site.name: 1 to 80 characters, no control characters"),
            Self::BadLanguage(m) => write!(f, "languages: {m}"),
            Self::DefaultNotListed(l) => write!(f, "default_language {l:?} is not in languages"),
            Self::DnsRecords(m) => write!(f, "dns.records: {m}"),
            Self::AcmeMethod(m) => write!(f, "acme.method: {m}"),
            Self::CspMode(m) => write!(f, "csp.mode: {m}"),
            Self::CspPolicy => write!(
                f,
                "csp.policy: 1 to 1000 characters of A-Za-z0-9 ;:'*._/=+,%-"
            ),
            Self::SiteCollision { a, b, name, path } => {
                write!(f, "sites {a:?} and {b:?} both claim {name}{path}")
            }
            Self::NestedShadowed {
                inner,
                outer,
                segment,
            } => write!(
                f,
                "site {inner:?} lies below site {outer:?} at {segment:?}, a name {outer:?} generates itself"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

/// A failed build.
#[derive(Debug)]
pub enum SiteError {
    /// `site.toml` (or the set of sites) was refused.
    Config(Vec<ConfigError>),
    /// Problems of the source or of the pages that stop the build (all listed).
    Problems(Vec<String>),
    /// The output folder exists and is not empty, or is not a folder.
    OutputNotEmpty(String),
    /// The file system said no.
    Io(std::io::Error),
}

impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(v) => {
                for e in v {
                    writeln!(f, "config: {e}")?;
                }
                Ok(())
            }
            Self::Problems(v) => {
                for e in v {
                    writeln!(f, "{e}")?;
                }
                Ok(())
            }
            Self::OutputNotEmpty(p) => write!(f, "output folder {p} exists and is not empty"),
            Self::Io(e) => write!(f, "file system: {e}"),
        }
    }
}

impl std::error::Error for SiteError {}

impl From<std::io::Error> for SiteError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
