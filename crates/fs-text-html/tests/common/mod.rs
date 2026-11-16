//! Helpers of the tests.
#![allow(dead_code)]
use fs_text::import_markdown;
use fs_text_html::{HtmlExporter, HtmlPage};

pub fn page(md: &str) -> HtmlPage {
    HtmlExporter::default().render(&import_markdown(md))
}

pub fn html(md: &str) -> String {
    page(md).body
}
