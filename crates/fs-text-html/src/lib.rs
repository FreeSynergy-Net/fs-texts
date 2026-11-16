//! `fs-text-html`: the text model of `fs-text` as HTML (ADR 0030, 0084, 0086).
//!
//! Pattern map: see `DESIGN.md`. Entry points: [`HtmlExporter`] (the [`fs_text::Exporter`] for
//! HTML and `render` with page data), [`frame::render_page`] (page frame with a short
//! navigation) and [`pass`] (user files copied byte for byte, behind path and media checks).
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions
)]

pub mod escape;
pub mod exporter;
pub mod frame;
pub mod nav;
pub mod page;
pub mod pass;
pub mod raw;
pub mod report;
pub mod resolve;
pub mod slug;
pub mod state;
pub mod theme;
pub mod url;
pub mod words;

mod inline;
mod leaf;
mod render;
mod table;
mod tree;

pub use exporter::{HtmlExporter, HtmlExporterBuilder};
pub use frame::{render_page, Alternate, Csp, Frame};
pub use nav::{NavNode, Navigation};
pub use page::{HtmlPage, TocEntry};
pub use raw::RawHtmlPolicy;
pub use report::{Problem, Report};
pub use resolve::{AsWritten, LinkResolver, LinkTarget, LocalMedia, MediaResolver, MediaTarget};
pub use words::Words;
