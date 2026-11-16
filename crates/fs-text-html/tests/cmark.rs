//! Counter-check against pulldown-cmark's HTML (dev-dependency only).
//!
//! Both outputs are cut into tokens and compared as a normalised sequence. Wanted differences,
//! named and removed by the normaliser: heading `id`, `class` (code language, alignment,
//! callouts, tasks), `scope`, `loading`, `style`, the `<label>` around the text of a task, the
//! form of the task box, `<br />` against `<br>`, and the way a table cell is aligned.
mod common;
use common::html;
use pulldown_cmark::{html::push_html, Options, Parser};

fn tokens(h: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = h;
    let push_text = |t: &str, out: &mut Vec<String>| {
        let t = t
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        if !t.is_empty() {
            out.push(t);
        }
    };
    while let Some(i) = rest.find('<') {
        push_text(&rest[..i], &mut out);
        let j = rest[i..].find('>').expect("tag end") + i;
        let tag = rest[i + 1..j].trim_end_matches('/').trim();
        rest = &rest[j + 1..];
        let name = tag.split_whitespace().next().unwrap_or("");
        match name {
            "label" | "/label" => {}
            "input" => out.push(if tag.contains("checked") {
                "[x]".into()
            } else {
                "[ ]".into()
            }),
            _ => {
                let mut t = format!("<{name}");
                for key in ["href", "src", "alt", "start"] {
                    if let Some(p) = tag.find(&format!(" {key}=\"")) {
                        let v = &tag[p + key.len() + 3..];
                        t.push_str(&format!(" {key}={}", &v[..v.find('"').unwrap_or(v.len())]));
                    }
                }
                out.push(t + ">");
            }
        }
    }
    push_text(rest, &mut out);
    out
}

fn ours_vs_cmark(md: &str) {
    let mut o = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    o.remove(Options::ENABLE_SMART_PUNCTUATION);
    let mut theirs = String::new();
    push_html(&mut theirs, Parser::new_ext(md, o));
    assert_eq!(
        tokens(&html(md)),
        tokens(&theirs),
        "markdown: {md:?}\nours:   {}\ntheirs: {theirs}",
        html(md)
    );
}

const CASES: &[&str] = &[
    "plain text",
    "a & b < c > d \"q\" 'r'",
    "# H1\n\n## H2 *em*\n\n### `code` heading",
    "Setext\n======\n\nSub\n---",
    "para one\n\npara two\nsecond line",
    "hard  \nbreak",
    "*em* _em_ **strong** __strong__ ***both***",
    "~~strike~~ and ~also~",
    "`code` and ``co`de``",
    "[link](https://a.b/c \"title\") and <https://auto.link>",
    "[ref][r]\n\n[r]: https://x.y/z",
    "![alt text](img.png \"t\")",
    "[![alt](i.png)](https://a.b)",
    "- a\n- b\n- c",
    "* a\n\n* b",
    "1. one\n2. two\n3. three",
    "5. five\n6. six",
    "- a\n  - b\n    - c\n- d",
    "1. a\n   - b\n2. c",
    "- [ ] todo\n- [x] done",
    "> quote\n> more\n\n> second",
    "> a\n>\n> > nested",
    "- item\n\n  second paragraph\n- next",
    "```\nplain code\n```",
    "```rust\nfn x() { 1 < 2 }\n```",
    "    indented code\n    more",
    "---\n\ntext\n\n***",
    "| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |",
    "| a | b |\n|:--|--:|\n| **x** | `y` |",
    "line with \\* escaped \\_ chars",
    "a\\\nb",
    "# Title\n\ntext with [link](a/b.md) and ![i](c.png)",
    "1. a\n\n   b\n\n2. c",
    "- a\n\n      code in item",
    "text <kbd>Ctrl</kbd> text",
];

#[test]
fn commonmark_and_gfm_samples_agree() {
    for md in CASES {
        ours_vs_cmark(md);
    }
}

#[test]
fn the_real_documentation_samples_agree_where_they_use_no_extension() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fs-text/tests/corpus");
    let mut n = 0;
    for e in std::fs::read_dir(dir).expect("corpus") {
        let p = e.expect("entry").path();
        let md = std::fs::read_to_string(&p).expect("utf-8");
        // callouts, footnotes, front matter, raw HTML, wiki links: ours differ by design
        let tag = md.contains("</")
            || md.contains("<!--")
            || md
                .match_indices('<')
                .any(|(i, _)| md[i + 1..].starts_with(|c: char| c.is_ascii_alphabetic()));
        if tag || md.starts_with("---") || ["[!", "[^", "[[", "!!!"].iter().any(|x| md.contains(x))
        {
            continue;
        }
        println!("{}", p.display());
        ours_vs_cmark(&md);
        n += 1;
    }
    println!("compared {n} corpus files");
}
