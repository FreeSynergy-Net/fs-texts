//! The page frame: `<html>`, `<head>`, header, short navigation, main, footer.
//!
//! Pattern: Template method — a fixed sequence of parts around the body of a page; the
//! pieces that differ (head data, navigation, language bar, CSP) come from [`Frame`]. No
//! script, no external resource. The CSP in a `<meta>` is a setting ([`Csp`]).

use crate::escape::{attr, attr_string, text_string};
use crate::nav::Navigation;
use crate::page::HtmlPage;
use crate::words::{autonym, Words};

/// The `id` of the `<main>` element every page of the frame has (the target of the skip link).
/// Such ids of the frame are anchors a link may point to; they are not in a page's body.
pub const FRAME_MAIN_ID: &str = "main";

/// The Content-Security-Policy written into the generated pages.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Csp {
    /// `default-src 'self'` with scripts, plug-ins, `<base>` and forms switched off.
    #[default]
    Strict,
    /// A policy of the caller, written as given.
    Custom(String),
    /// No policy in the page (the server sends one).
    Off,
}

impl Csp {
    /// The `content` of the meta tag, if any.
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        match self {
            Csp::Strict => Some(
                "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'",
            ),
            Csp::Custom(s) => Some(s),
            Csp::Off => None,
        }
    }
}

/// A language variant of the page, for `hreflang` and the language bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alternate {
    /// Language tag.
    pub lang: String,
    /// Absolute address of the variant.
    pub href: String,
}

/// Everything around the body of one page.
#[derive(Clone, Debug)]
pub struct Frame {
    /// Language of the page (`html lang`, words of the frame).
    pub lang: String,
    /// Name of the site.
    pub site_name: String,
    /// Absolute address of this page.
    pub canonical: Option<String>,
    /// All variants, this one included; the bar shows only with two or more.
    pub alternates: Vec<Alternate>,
    /// Address of the language choice (`x-default`).
    pub x_default: Option<String>,
    /// Address of the style sheet.
    pub stylesheet: String,
    /// The policy.
    pub csp: Csp,
    /// The tree of the site in this language.
    pub nav: Navigation,
    /// Address of this page in `nav`.
    pub current: String,
    /// Plain footer text.
    pub footer: Option<String>,
}

/// The whole page.
#[must_use]
pub fn render_page(f: &Frame, page: &HtmlPage) -> String {
    let w = Words::for_lang(&f.lang);
    let mut o = String::from("<!DOCTYPE html>\n<html lang=\"");
    attr(&f.lang, &mut o);
    o.push_str("\">\n<head>\n<meta charset=\"utf-8\">\n");
    if let Some(c) = f.csp.content() {
        o.push_str(&format!(
            "<meta http-equiv=\"Content-Security-Policy\" content=\"{}\">\n",
            attr_string(c)
        ));
    }
    o.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    let title = page.title.as_deref().filter(|t| !t.is_empty());
    let full = match title {
        Some(t) if t != f.site_name => format!("{t} \u{2013} {}", f.site_name),
        _ => f.site_name.clone(),
    };
    o.push_str(&format!("<title>{}</title>\n", text_string(&full)));
    if let Some(d) = page.description.as_deref() {
        o.push_str(&format!(
            "<meta name=\"description\" content=\"{}\">\n",
            attr_string(d)
        ));
    }
    if let Some(c) = &f.canonical {
        o.push_str(&format!(
            "<link rel=\"canonical\" href=\"{}\">\n",
            attr_string(c)
        ));
    }
    for a in &f.alternates {
        o.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"{}\" href=\"{}\">\n",
            attr_string(&a.lang),
            attr_string(&a.href)
        ));
    }
    if let Some(x) = &f.x_default {
        o.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"x-default\" href=\"{}\">\n",
            attr_string(x)
        ));
    }
    o.push_str(&format!(
        "<link rel=\"stylesheet\" href=\"{}\">\n</head>\n<body>\n",
        attr_string(&f.stylesheet)
    ));
    o.push_str(&format!(
        "<a class=\"skip\" href=\"#{FRAME_MAIN_ID}\">{}</a>\n",
        text_string(w.skip)
    ));
    header(f, &w, &mut o);
    o.push_str(&format!(
        "<div class=\"layout\">\n<nav class=\"side\" aria-label=\"{}\">\n{}</nav>\n",
        attr_string(w.nav),
        f.nav.render(&f.current)
    ));
    o.push_str(&format!("<main id=\"{FRAME_MAIN_ID}\">\n"));
    crumbs(f, &w, &mut o);
    if page.top_headings == 0 {
        if let Some(t) = title {
            o.push_str(&format!("<h1>{}</h1>\n", text_string(t)));
        }
    }
    o.push_str(&page.body);
    o.push_str("</main>\n</div>\n");
    if let Some(t) = &f.footer {
        o.push_str(&format!("<footer><p>{}</p></footer>\n", text_string(t)));
    }
    o.push_str("</body>\n</html>\n");
    o
}

fn header(f: &Frame, w: &Words, o: &mut String) {
    o.push_str(&format!(
        "<header>\n<p class=\"site\">{}</p>\n",
        text_string(&f.site_name)
    ));
    if f.alternates.len() > 1 {
        o.push_str(&format!(
            "<nav class=\"langs\" aria-label=\"{}\"><ul>",
            attr_string(w.languages)
        ));
        for a in &f.alternates {
            let cur = if a.lang == f.lang {
                " aria-current=\"true\""
            } else {
                ""
            };
            o.push_str(&format!(
                "<li><a href=\"{}\" hreflang=\"{}\" lang=\"{}\"{cur}>{}</a></li>",
                attr_string(&a.href),
                attr_string(&a.lang),
                attr_string(&a.lang),
                text_string(autonym(&a.lang))
            ));
        }
        o.push_str("</ul></nav>\n");
    }
    o.push_str("</header>\n");
}

fn crumbs(f: &Frame, w: &Words, o: &mut String) {
    let trail = f.nav.trail(&f.current);
    if trail.len() < 2 {
        return;
    }
    o.push_str(&format!(
        "<nav class=\"crumbs\" aria-label=\"{}\"><ol>",
        attr_string(w.breadcrumb)
    ));
    for (i, (t, h)) in trail.iter().enumerate() {
        if i + 1 == trail.len() {
            o.push_str(&format!(
                "<li aria-current=\"page\">{}</li>",
                text_string(t)
            ));
        } else {
            o.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>",
                attr_string(h),
                text_string(t)
            ));
        }
    }
    o.push_str("</ol></nav>\n");
}
