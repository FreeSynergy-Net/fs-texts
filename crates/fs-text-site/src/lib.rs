//! `fs-text-site`: a folder of text and user files in, a static website folder out.
//!
//! Core 2 of the website export (stage 1). Pattern map: see `DESIGN.md`. Entry points:
//! [`FolderSource`] (the adapter of the port [`SiteSource`]), [`build`] (one site), [`build_all`]
//! (all sites of a domain, checked together) and [`SiteConfig`] (`site.toml`, schema 1 and 2).
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::format_push_string,
    clippy::too_many_lines
)]

pub mod build;
pub mod config;
pub mod error;
pub mod folder;
pub mod gen;
pub mod host;
pub mod index;
pub mod outfs;
pub mod report;
pub mod sites;
pub mod source;
pub mod tree;
pub mod words;

pub use build::{build, build_all, BuildOptions};
pub use config::{CspMode, DnsRecord, Lang, SiteConfig};
pub use error::{ConfigError, SiteError};
pub use folder::FolderSource;
pub use host::{Host, PathPrefix};
pub use report::{BuildReport, Finding, Gate, Policy};
pub use sites::SiteSet;
pub use source::{Area, FileEntry, Listing, PageEntry, SiteSource};
