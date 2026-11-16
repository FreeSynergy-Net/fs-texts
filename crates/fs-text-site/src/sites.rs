//! Several sites on one domain: the checks across sites.
//!
//! Pattern: Specification over a set (like `PassSet` for paths). Each site claims
//! `(name, path)` for every name it answers to. Two sites may not claim the same pair; one may
//! lie below the other (the landing page at `/`, the wiki at `/wiki`) only if the inner prefix
//! does not sit on a name the outer site generates itself. The outer site then reserves that
//! name against its own pass-through files, so nothing of it is ever overwritten.

use crate::config::SiteConfig;
use crate::error::ConfigError;

/// Names at the top of every site's output that the builder writes itself.
pub const GENERATED_TOP: [&str; 7] = [
    "site.toml",
    "_fs",
    "index.html",
    "sitemap.xml",
    "robots.txt",
    "404.html",
    "site-manifest.json",
];

/// A checked set of sites.
#[derive(Debug)]
pub struct SiteSet {
    sites: Vec<SiteConfig>,
}

impl SiteSet {
    /// Check all collisions; every problem is listed.
    pub fn new(sites: Vec<SiteConfig>) -> Result<Self, Vec<ConfigError>> {
        let mut errs = Vec::new();
        for (i, a) in sites.iter().enumerate() {
            for b in &sites[i + 1..] {
                check_pair(a, b, &mut errs);
            }
        }
        if errs.is_empty() {
            Ok(Self { sites })
        } else {
            Err(errs)
        }
    }

    /// The sites.
    #[must_use]
    pub fn sites(&self) -> &[SiteConfig] {
        &self.sites
    }

    /// The first segments (below the prefix of `site`) at which other sites on a shared name
    /// lie inside `site`: the pass-through of `site` may not use them.
    #[must_use]
    pub fn nested_below(&self, site: &SiteConfig) -> Vec<String> {
        let mut out = Vec::new();
        for o in self.sites.iter().filter(|o| o.id != site.id) {
            if shares_name(site, o)
                && site.path_prefix.is_prefix_of(&o.path_prefix)
                && o.path_prefix != site.path_prefix
            {
                let seg = o.path_prefix.segments()[site.path_prefix.segments().len()].clone();
                if !out.contains(&seg) {
                    out.push(seg);
                }
            }
        }
        out.sort();
        out
    }
}

fn shares_name(a: &SiteConfig, b: &SiteConfig) -> bool {
    a.names.iter().any(|n| b.names.contains(n))
}

fn generated_top(s: &SiteConfig) -> Vec<String> {
    let mut v: Vec<String> = GENERATED_TOP.iter().map(|x| (*x).to_string()).collect();
    v.extend(s.languages.iter().map(|l| l.as_str().to_string()));
    v
}

fn check_pair(a: &SiteConfig, b: &SiteConfig, errs: &mut Vec<ConfigError>) {
    if a.id == b.id {
        errs.push(ConfigError::DuplicateId(a.id.clone()));
        return;
    }
    let Some(name) = a.names.iter().find(|n| b.names.contains(n)) else {
        return;
    };
    let path = |s: &SiteConfig| {
        if s.path_prefix.is_root() {
            "/".to_string()
        } else {
            s.url_prefix()
        }
    };
    if a.path_prefix == b.path_prefix {
        errs.push(ConfigError::SiteCollision {
            a: a.id.clone(),
            b: b.id.clone(),
            name: name.as_str().to_string(),
            path: path(a),
        });
        return;
    }
    for (outer, inner) in [(a, b), (b, a)] {
        if outer.path_prefix.is_prefix_of(&inner.path_prefix) {
            let seg = &inner.path_prefix.segments()[outer.path_prefix.segments().len()];
            if generated_top(outer)
                .iter()
                .any(|g| g.eq_ignore_ascii_case(seg))
            {
                errs.push(ConfigError::NestedShadowed {
                    inner: inner.id.clone(),
                    outer: outer.id.clone(),
                    segment: seg.clone(),
                });
            }
        }
    }
}
