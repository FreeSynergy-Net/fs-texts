//! The HTML exporter: our block document in, HTML out.
//!
//! Pattern: Adapter (of the `Exporter` target interface) + Builder for its settings. The
//! strategies (`LinkResolver`, `MediaResolver`, `RawHtmlPolicy`) are handed in.

use crate::page::HtmlPage;
use crate::raw::RawHtmlPolicy;
use crate::render::render_blocks;
use crate::resolve::{AsWritten, LinkResolver, LocalMedia, MediaResolver};
use crate::state::Env;
use crate::words::Words;
use fs_text::{Block, Document, Exporter};

/// Writes HTML. `Default` is the standard: links as written, relative media only, the raw HTML
/// allow-list, English words.
pub struct HtmlExporter {
    links: Box<dyn LinkResolver>,
    media: Box<dyn MediaResolver>,
    raw: RawHtmlPolicy,
    words: Words,
}

impl Default for HtmlExporter {
    fn default() -> Self {
        HtmlExporterBuilder::default().build()
    }
}

/// Builder of an [`HtmlExporter`].
pub struct HtmlExporterBuilder {
    links: Box<dyn LinkResolver>,
    media: Box<dyn MediaResolver>,
    raw: RawHtmlPolicy,
    lang: String,
}

impl Default for HtmlExporterBuilder {
    fn default() -> Self {
        Self {
            links: Box::new(AsWritten),
            media: Box::new(LocalMedia),
            raw: RawHtmlPolicy::default(),
            lang: "en".into(),
        }
    }
}

impl HtmlExporterBuilder {
    /// Where links point.
    #[must_use]
    pub fn links(mut self, r: impl LinkResolver + 'static) -> Self {
        self.links = Box::new(r);
        self
    }
    /// Where images point.
    #[must_use]
    pub fn media(mut self, r: impl MediaResolver + 'static) -> Self {
        self.media = Box::new(r);
        self
    }
    /// What happens to raw HTML.
    #[must_use]
    pub fn raw(mut self, p: RawHtmlPolicy) -> Self {
        self.raw = p;
        self
    }
    /// Language of the generated words (footnote list, task labels).
    #[must_use]
    pub fn lang(mut self, lang: &str) -> Self {
        self.lang = lang.to_string();
        self
    }
    /// The exporter.
    #[must_use]
    pub fn build(self) -> HtmlExporter {
        HtmlExporter {
            links: self.links,
            media: self.media,
            raw: self.raw,
            words: Words::for_lang(&self.lang),
        }
    }
}

impl HtmlExporter {
    /// A document as a page: fragment plus title, headings, used links and media, report.
    #[must_use]
    pub fn render(&self, doc: &Document) -> HtmlPage {
        self.render_blocks(&doc.blocks)
    }
    /// A block sequence as a page.
    #[must_use]
    pub fn render_blocks(&self, blocks: &[Block]) -> HtmlPage {
        let env = Env {
            links: self.links.as_ref(),
            media: self.media.as_ref(),
            raw: self.raw,
            words: self.words,
        };
        render_blocks(&env, blocks)
    }
}

impl Exporter for HtmlExporter {
    fn export(&self, doc: &Document) -> String {
        self.render(doc).body
    }
    fn export_blocks(&self, blocks: &[Block]) -> String {
        self.render_blocks(blocks).body
    }
}
