//! A table block as `<table>`.
//!
//! Pattern: Template method — split the block text into rows and cells, write each cell with
//! the inline writer, head row first. Alignment is a class (a `style` attribute would need a
//! looser CSP).

use crate::inline;
use crate::state::{Env, PageState};
use fs_text::{Align, Block};

fn class(a: Align) -> &'static str {
    match a {
        Align::None => "",
        Align::Left => " class=\"al-l\"",
        Align::Center => " class=\"al-c\"",
        Align::Right => " class=\"al-r\"",
    }
}

pub(crate) fn render(env: &Env, st: &mut PageState, b: &Block, align: &[Align], out: &mut String) {
    let chars: Vec<char> = b.text.chars().collect();
    let marks = b.marks.as_slice();
    // rows end at '\n', cells at '\t'
    let mut rows: Vec<Vec<(usize, usize)>> = vec![Vec::new()];
    let mut start = 0;
    for i in 0..=chars.len() {
        let end_row = i == chars.len() || chars[i] == '\n';
        if end_row || chars[i] == '\t' {
            if let Some(r) = rows.last_mut() {
                r.push((start, i));
            }
            start = i + 1;
            if end_row && i < chars.len() {
                rows.push(Vec::new());
            }
        }
    }
    out.push_str("<table>\n");
    for (n, row) in rows.iter().enumerate() {
        if n == 0 {
            out.push_str("<thead>\n");
        } else if n == 1 {
            out.push_str("<tbody>\n");
        }
        out.push_str("<tr>");
        for (c, range) in row.iter().enumerate() {
            let tag = if n == 0 { "th" } else { "td" };
            let scope = if n == 0 { " scope=\"col\"" } else { "" };
            let al = class(align.get(c).copied().unwrap_or(Align::None));
            out.push_str(&format!("<{tag}{scope}{al}>"));
            inline::render(env, st, &chars, marks, *range, out);
            out.push_str(&format!("</{tag}>"));
        }
        out.push_str("</tr>\n");
        if n == 0 {
            out.push_str("</thead>\n");
        }
    }
    if rows.len() > 1 {
        out.push_str("</tbody>\n");
    }
    out.push_str("</table>\n");
}
