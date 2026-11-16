//! Marks over a stretch of block text, written as correctly nested inline HTML.
//!
//! Pattern: Visitor (one `match` per mark kind, outside the data) over a sweep along the text.
//! A mark that ends while a younger one is still open closes it first and opens it again, so
//! tags are always nested. Layer marks (passage, variable, explanation) are not exported
//! (ADR 0042): the original text stays.

use crate::atoms::{footnote, image, link_tag};
use crate::escape;
use crate::raw::Cleaner;
use crate::report::Problem;
use crate::state::{Env, PageState};
use fs_text::markdown::HARD_BREAK;
use fs_text::{Mark, MarkKind, MarkValue};

struct Open {
    end: usize,
    close: &'static str,
    /// Is this the open link (a link may not hold another link)?
    link: bool,
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
            | MarkKind::Html => m.end > a && m.start < b && m.start < m.end,
            MarkKind::Image if m.start == m.end => m.start >= a && m.start <= b,
            MarkKind::Image => m.end > a && m.start < b,
            MarkKind::FootnoteRef => m.start >= a && m.start <= b,
            MarkKind::Variable => {
                if m.start >= a && m.start <= b {
                    if let MarkValue::Variable { name, .. } = &m.value {
                        st.report
                            .note(Problem::UnfilledVariable { name: name.clone() });
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

fn open(
    env: &Env,
    st: &mut PageState,
    stack: &mut Vec<Open>,
    clean: &Cleaner,
    m: &Mark,
    out: &mut String,
) {
    let (tag, close): (String, &'static str) = match m.kind {
        MarkKind::Strong => ("<strong>".into(), "</strong>"),
        MarkKind::Emphasis => ("<em>".into(), "</em>"),
        MarkKind::Code => ("<code>".into(), "</code>"),
        MarkKind::Strike => ("<del>".into(), "</del>"),
        _ => {
            if stack.iter().any(|o| o.link) {
                return;
            }
            let Some(t) = link_tag(env, st, m) else {
                return;
            };
            (t, "</a>")
        }
    };
    out.push_str(&tag);
    stack.push(Open {
        end: m.end,
        close,
        link: m.kind == MarkKind::Link,
        tag,
        san: clean.depth(),
    });
}
