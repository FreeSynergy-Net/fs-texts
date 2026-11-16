//! `[build]` of `site.toml`: rules of the build itself.
//!
//! Pattern: Value object (checked on construction in `raw::build_rules`).

/// `[build] dead_links`: what a link to a page, file or anchor of the site that does not exist does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeadLinks {
    /// Stop the build (default).
    Error,
    /// Build, list all of them in the report, exit code 0 with a warning.
    List,
}

/// `[build]`: rules of the build itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildRules {
    /// What a dead link does.
    pub dead_links: DeadLinks,
    /// Generate an overview page for a folder that has no `index.md` (default: yes).
    pub folder_index: bool,
}

impl Default for BuildRules {
    fn default() -> Self {
        Self {
            dead_links: DeadLinks::Error,
            folder_index: true,
        }
    }
}

/// The policy of a folder rule for the page at `path` (below the language), if one applies:
/// the longest folder that is a prefix of the address.
#[must_use]
pub fn area_policy<'a>(cfg: &'a crate::config::SiteConfig, path: &[String]) -> Option<&'a str> {
    cfg.csp_areas
        .iter()
        .filter(|a| a.policy.is_some() && path.starts_with(a.path.segments()))
        .max_by_key(|a| a.path.segments().len())
        .and_then(|a| a.policy.as_deref())
}
