//! The atoms of inline text: link tag, image, footnote reference.
//!
//! Pattern: Strategy client — the address of a link or image comes from the resolvers of the
//! [`Env`]; unsafe or unknown targets become plain text and a report entry. Part of `inline`.

use crate::escape;
use crate::report::Problem;
use crate::state::{Env, PageState};
use crate::url::safe_link;
use fs_text::markdown::HARD_BREAK;
use fs_text::{Mark, MarkValue};

pub(super) fn link_tag(env: &Env, st: &mut PageState, m: &Mark) -> Option<String> {
    let l = m.link()?;
    if !safe_link(&l.path) {
        st.report.note(Problem::UnsafeLink {
            path: l.path.clone(),
        });
        return None;
    }
    let href = match env.links.resolve(l) {
        crate::LinkTarget::Href(h) if safe_link(&h) => h,
        crate::LinkTarget::Href(h) => {
            st.report.note(Problem::UnsafeLink { path: h });
            return None;
        }
        crate::LinkTarget::Missing | crate::LinkTarget::NotPublic => {
            st.report.note(Problem::DeadLink {
                path: l.path.clone(),
            });
            return None;
        }
    };
    st.used_link(&href);
    let mut t = String::from("<a href=\"");
    escape::attr(href.trim(), &mut t);
    t.push('"');
    if let Some(title) = l.title.as_deref().filter(|s| !s.is_empty()) {
        t.push_str(" title=\"");
        escape::attr(title, &mut t);
        t.push('"');
    }
    t.push('>');
    Some(t)
}

pub(super) fn image(
    env: &Env,
    st: &mut PageState,
    chars: &[char],
    m: &Mark,
    (a, b): (usize, usize),
    out: &mut String,
) {
    let Some(l) = m.link() else { return };
    let alt: String = chars[m.start.max(a)..m.end.min(b)].iter().collect();
    let Some(t) = env.media.resolve(l).filter(|t| safe_link(&t.src)) else {
        st.report.note(Problem::UnsafeMedia {
            src: l.path.clone(),
        });
        escape::text(&alt, out);
        return;
    };
    if alt.trim().is_empty() {
        st.report.note(Problem::MissingAlt {
            src: l.path.clone(),
        });
    }
    st.used_media(&t.src);
    out.push_str("<img src=\"");
    escape::attr(&t.src, out);
    out.push_str("\" alt=\"");
    escape::attr(&alt.replace(HARD_BREAK, " "), out);
    out.push('"');
    if let (Some(w), Some(h)) = (t.width, t.height) {
        out.push_str(&format!(" width=\"{w}\" height=\"{h}\""));
    }
    out.push_str(" loading=\"lazy\">");
}

pub(super) fn footnote(st: &mut PageState, m: &Mark, out: &mut String) {
    let MarkValue::Name(label) = &m.value else {
        return;
    };
    if let Some((n, k)) = st.note_number(label) {
        let id = if k == 1 {
            format!("fnref-{n}")
        } else {
            format!("fnref-{n}-{k}")
        };
        out.push_str(&format!(
            "<sup class=\"fnref\"><a id=\"{id}\" href=\"#fn-{n}\">{n}</a></sup>"
        ));
    } else {
        st.report.note(Problem::FootnoteUndefined {
            label: label.clone(),
        });
        out.push_str("<sup class=\"fnref\">?</sup>");
    }
}
