//! `fs-text-html`: the text model of `fs-text` as HTML (ADR 0030, 0084, 0086).
//!
//! Pattern map: see `DESIGN.md`. Entry points: [`HtmlExporter`] (the [`fs_text::Exporter`] for
//! HTML and `render` with page data), [`frame::render_page`] (page frame with a short
//! navigation) and [`pass`] (user files copied byte for byte, behind path and media checks).
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// Pedantic lints that would only add noise here: results of getters are used by callers, names
// repeat their module on purpose, markup is written with `push_str(&format!(..))` (one short
// line per tag), and `a`/`b`/`s` are the usual names of a range and a source string.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::format_push_string,
    clippy::many_single_char_names
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

mod atoms;
mod inline;
mod leaf;
mod rawtag;
mod render;
mod render_parts;
mod table;
mod tree;

pub use exporter::{HtmlExporter, HtmlExporterBuilder};
pub use frame::{render_page, Alternate, Csp, Frame, FRAME_MAIN_ID};
pub use nav::{NavNode, Navigation};
pub use page::{HtmlPage, TocEntry};
pub use raw::RawHtmlPolicy;
pub use report::{Problem, Report};
pub use resolve::{AsWritten, LinkResolver, LinkTarget, LocalMedia, MediaResolver, MediaTarget};
pub use words::Words;
