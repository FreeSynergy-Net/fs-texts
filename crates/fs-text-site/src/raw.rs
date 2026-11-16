//! The raw, typed form of `site.toml` (serde, unknown keys refused) and the small checks of its
//! parts (`dns`, `csp`).
//!
//! Pattern: Builder input. [`crate::config::SiteConfig`] is made from these; nothing outside
//! reads them.

use crate::config::{CspArea, CspMode, DnsRecord};
use crate::error::ConfigError;
use crate::host::PathPrefix;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawNames {
    pub(crate) primary: String,
    pub(crate) aliases: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDns {
    pub(crate) records: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAcme {
    pub(crate) method: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawArea {
    pub(crate) path: String,
    pub(crate) mode: Option<String>,
    pub(crate) policy: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawCsp {
    pub(crate) mode: Option<String>,
    pub(crate) policy: Option<String>,
    pub(crate) area: Option<Vec<RawArea>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSite {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) path_prefix: Option<String>,
    pub(crate) languages: Option<Vec<String>>,
    pub(crate) default_language: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Raw {
    #[allow(dead_code)]
    pub(crate) schema: i64,
    pub(crate) purpose: String,
    pub(crate) site: Option<RawSite>,
    pub(crate) names: RawNames,
    pub(crate) dns: Option<RawDns>,
    pub(crate) acme: RawAcme,
    pub(crate) csp: Option<RawCsp>,
}

pub(crate) fn dns_records(d: Option<RawDns>) -> Result<Vec<DnsRecord>, ConfigError> {
    let Some(d) = d else {
        return Ok(vec![DnsRecord::A, DnsRecord::Aaaa]);
    };
    if d.records.is_empty() {
        return Err(ConfigError::DnsRecords("must not be empty".into()));
    }
    let mut out = Vec::new();
    for r in &d.records {
        let t = match r.as_str() {
            "A" => DnsRecord::A,
            "AAAA" => DnsRecord::Aaaa,
            "CAA" => DnsRecord::Caa,
            _ => return Err(ConfigError::DnsRecords("only A, AAAA, CAA".into())),
        };
        if out.contains(&t) {
            return Err(ConfigError::DnsRecords("a type twice".into()));
        }
        out.push(t);
    }
    Ok(out)
}

fn mode_of(m: Option<&str>) -> Result<CspMode, ConfigError> {
    match m {
        None | Some("static") => Ok(CspMode::Static),
        Some("passthrough") => Ok(CspMode::Passthrough),
        Some(_) => Err(ConfigError::CspMode("static or passthrough".into())),
    }
}

fn policy_ok(p: Option<&String>) -> Result<(), ConfigError> {
    if let Some(p) = p {
        let ok = (1..=1000).contains(&p.len())
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || " ;:'*._/=+,%-".contains(c));
        if !ok {
            return Err(ConfigError::CspPolicy);
        }
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
pub(crate) fn csp(
    c: Option<RawCsp>,
    v1: bool,
) -> Result<(CspMode, Option<String>, Vec<CspArea>), ConfigError> {
    let Some(c) = c else {
        return Ok((CspMode::Static, None, Vec::new()));
    };
    let mode = mode_of(c.mode.as_deref())?;
    policy_ok(c.policy.as_ref())?;
    let mut areas: Vec<CspArea> = Vec::new();
    if let Some(list) = c.area {
        if v1 {
            return Err(ConfigError::CspMode("[[csp.area]] needs schema = 2".into()));
        }
        if list.len() > 8 {
            return Err(ConfigError::CspMode("more than 8 areas".into()));
        }
        for a in list {
            let path = PathPrefix::parse(&a.path)?;
            if path.is_root() {
                return Err(ConfigError::CspMode(
                    "an area is a folder; the whole site is [csp] itself".into(),
                ));
            }
            policy_ok(a.policy.as_ref())?;
            if areas.iter().any(|x| x.path == path) {
                return Err(ConfigError::CspMode(format!("area {} twice", a.path)));
            }
            areas.push(CspArea {
                path,
                mode: mode_of(a.mode.as_deref())?,
                policy: a.policy,
            });
        }
        areas.sort_by(|a, b| a.path.cmp(&b.path));
    }
    Ok((mode, c.policy, areas))
}
