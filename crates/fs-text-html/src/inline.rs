//! Marks over a stretch of block text, written as correctly nested inline HTML.
//!
//! Pattern: Visitor (one `match` per mark kind, outside the data) over a sweep along the text.
//! A mark that ends while a younger one is still open closes it first and opens it again, so
//! tags are always nested. Layer marks (passage, variable, explanation) are not exported
//! (ADR 0042): the original text stays.

use crate::escape;
use crate::raw::Cleaner;
use crate::report::Problem;
use crate::state::{Env, PageState};
use crate::url::safe_link;
use fs_text::markdown::HARD_BREAK;
use fs_text::{Mark, MarkKind, MarkValue};

struct Open {
    end: usize,
    close: &'static str,
    tag: String,
    san: usize,
}

/// Write `chars[a..b]` with the `marks` that touch it (sorted by start, longest first).
pub(crate) fn render(
    env: &Env,
    st: &mut PageState,
    chars: &[char],
    marks: &[Mark],
    (a, b): (usize, usize),
    out: &mut String,
) {
    let live: Vec<&Mark> = marks
        .iter()
        .filter(|m| match m.kind {
            MarkKind::Strong
            | MarkKind::Emphasis
            | MarkKind::Code
            | MarkKind::Strike
            | MarkKind::Link
            | MarkKind::Image
            | MarkKind::Html => m.end > a && m.start < b && (m.start < m.end || m.kind == MarkKind::Image),
            MarkKind::FootnoteRef => m.start >= a && m.start <= b,
            MarkKind::Variable => {
                if m.start >= a && m.start <= b {
                    if let MarkValue::Variable { name, .. } = &m.value {
                        st.report.note(Problem::UnfilledVariable { name: name.clone() });
                    }
                }
                false
            }
            _ => false,
        })
        .collect();
    let mut clean = Cleaner::new(env.raw);
    let mut stack: Vec<Open> = Vec::new();
    let mut next = 0;
    let mut p = a;
    loop {
        let mut moved = false;
        close_ended(&mut stack, &mut clean, p, out);
        while next < live.len() && live[next].start.max(a) <= p {
            let m = live[next];
            next += 1;
            if m.start.max(a) < p {
                continue;
            }
            match m.kind {
                MarkKind::FootnoteRef => footnote(st, m, out),
                MarkKind::Image => {
                    image(env, st, chars, m, (a, b), out);
                    p = m.end.min(b).max(p);
                    moved = true;
                }
                MarkKind::Html => {
                    clean.set_floor(stack.last().map_or(0, |o| o.san));
                    let raw: String = chars[p..m.end.min(b)].iter().collect();
                    clean.feed(&raw, out, &mut st.report);
                    p = m.end.min(b);
                    moved = true;
                }
                _ => open(env, st, &mut stack, &clean, m, out),
            }
        }
        if moved {
            continue;
        }
        if p >= b {
            break;
        }
        close_ended(&mut stack, &mut clean, p, out);
        text_char(chars[p], out);
        p += 1;
    }
    while let Some(o) = stack.pop() {
        clean.close_to(o.san, out);
        out.push_str(o.close);
    }
    clean.close_to(0, out);
}

fn text_char(c: char, out: &mut String) {
    match c {
        HARD_BREAK => out.push_str("<br>"),
        c => escape::text(c.encode_utf8(&mut [0; 4]), out),
    }
}

fn close_ended(stack: &mut Vec<Open>, clean: &mut Cleaner, p: usize, out: &mut String) {
    let Some(low) = stack.iter().position(|o| o.end <= p) else {
        return;
    };
    let mut again = Vec::new();
    while stack.len() > low {
        let o = stack.pop().unwrap_or_else(|| unreachable!());
        clean.close_to(o.san, out);
        out.push_str(o.close);
        if o.end > p {
            again.push(o);
        }
    }
    for mut o in again.into_iter().rev() {
        out.push_str(&o.tag);
        o.san = clean.depth();
        stack.push(o);
    }
    clean.set_floor(stack.last().map_or(0, |o| o.san));
}

fn open(env: &Env, st: &mut PageState, stack: &mut Vec<Open>, clean: &Cleaner, m: &Mark, out: &mut String) {
    let (tag, close): (String, &'static str) = match m.kind {
        MarkKind::Strong => ("<strong>".into(), "</strong>"),
        MarkKind::Emphasis => ("<em>".into(), "</em>"),
        MarkKind::Code => ("<code>".into(), "</code>"),
        MarkKind::Strike => ("<del>".into(), "</del>"),
        _ => {
            if stack.iter().any(|o| o.close == "</a>") {
                return;
            }
            let Some(t) = link_tag(env, st, m) else { return };
            (t, "</a>")
        }
    };
    out.push_str(&tag);
    stack.push(Open { end: m.end, close, tag, san: clean.depth() });
}

fn link_tag(env: &Env, st: &mut PageState, m: &Mark) -> Option<String> {
    let l = m.link()?;
    if !safe_link(&l.path) {
        st.report.note(Problem::UnsafeLink { path: l.path.clone() });
        return None;
    }
    let href = match env.links.resolve(l) {
        crate::LinkTarget::Href(h) if safe_link(&h) => h,
        crate::LinkTarget::Href(h) => {
            st.report.note(Problem::UnsafeLink { path: h });
            return None;
        }
        crate::LinkTarget::Missing | crate::LinkTarget::NotPublic => {
            st.report.note(Problem::DeadLink { path: l.path.clone() });
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

fn image(env: &Env, st: &mut PageState, chars: &[char], m: &Mark, (a, b): (usize, usize), out: &mut String) {
    let Some(l) = m.link() else { return };
    let alt: String = chars[m.start.max(a)..m.end.min(b)].iter().collect();
    let Some(t) = env.media.resolve(l).filter(|t| safe_link(&t.src)) else {
        st.report.note(Problem::UnsafeMedia { src: l.path.clone() });
        escape::text(&alt, out);
        return;
    };
    if alt.trim().is_empty() {
        st.report.note(Problem::MissingAlt { src: l.path.clone() });
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

fn footnote(st: &mut PageState, m: &Mark, out: &mut String) {
    let MarkValue::Name(label) = &m.value else { return };
    match st.note_number(label) {
        Some((n, k)) => {
            let id = if k == 1 { format!("fnref-{n}") } else { format!("fnref-{n}-{k}") };
            out.push_str(&format!("<sup class=\"fnref\"><a id=\"{id}\" href=\"#fn-{n}\">{n}</a></sup>"));
        }
        None => {
            st.report.note(Problem::FootnoteUndefined { label: label.clone() });
            out.push_str("<sup class=\"fnref\">?</sup>");
        }
    }
}
