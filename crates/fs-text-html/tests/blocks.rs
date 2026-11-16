//! Blocks, containers and marks of the HTML exporter.
mod common;
use common::{html, page};
use fs_text::{import_markdown, Exporter};
use fs_text_html::{HtmlExporter, Problem};

#[test]
fn paragraph_heading_rule_and_escape() {
    assert_eq!(html("a < b & c > d"), "<p>a &lt; b &amp; c &gt; d</p>\n");
    assert_eq!(
        html("## Hi *there*"),
        "<h2 id=\"hi-there\">Hi <em>there</em></h2>\n"
    );
    assert_eq!(html("a\n\n---\n\nb"), "<p>a</p>\n<hr>\n<p>b</p>\n");
}

#[test]
fn heading_ids_are_ascii_and_unique() {
    let h = html("# Größe & Ärger\n\n# Größe & Ärger\n\n# 日本\n\n## main");
    assert!(h.contains("id=\"groesse-aerger\""), "{h}");
    assert!(h.contains("id=\"groesse-aerger-2\""), "{h}");
    assert!(h.contains("id=\"section\""), "{h}");
    assert!(h.contains("id=\"main-2\""), "reserved frame id: {h}");
    assert!(h.is_ascii() || !h.contains("id=\"é"));
}

#[test]
fn heading_cannot_take_a_footnote_id() {
    assert!(html("# fn-1").contains("id=\"h-fn-1\""));
}

#[test]
fn code_block_is_only_escaped() {
    let h = html("```rust\nlet a = \"<b>\";\n```");
    assert_eq!(
        h,
        "<pre><code class=\"language-rust\">let a = \"&lt;b&gt;\";\n</code></pre>\n"
    );
    let h = html("```x\" onload=\"y\nz\n```");
    assert!(h.contains("class=\"language-x\""), "{h}");
}

#[test]
fn marks_nest_properly_when_they_overlap() {
    use fs_text::{Block, BlockKind, Mark, MarkKind, MarkValue};
    let mut b = Block::new(BlockKind::Paragraph, "abcdef");
    b.marks
        .add(Mark::new(MarkKind::Strong, 0, 4, MarkValue::None));
    b.marks
        .add(Mark::new(MarkKind::Emphasis, 2, 6, MarkValue::None));
    let h = HtmlExporter::default().export_blocks(&[b]);
    assert_eq!(h, "<p><strong>ab<em>cd</em></strong><em>ef</em></p>\n");
}

#[test]
fn lists_tight_loose_ordered_nested_and_tasks() {
    assert_eq!(
        html("- a\n- b"),
        "<ul>\n<li>a\n</li>\n<li>b\n</li>\n</ul>\n"
    );
    let h = html("3. x\n4. y");
    assert!(h.starts_with("<ol start=\"3\">"), "{h}");
    let h = html("- a\n\n- b");
    assert!(h.contains("<li>\n<p>a</p>"), "loose: {h}");
    let h = html("- a\n  - b");
    assert!(h.contains("<li>a\n<ul>\n<li>b"), "{h}");
    let h = html("- [ ] open\n- [x] done");
    assert!(
        h.contains("<li class=\"task\"><label><input type=\"checkbox\" disabled> open</label>"),
        "{h}"
    );
    assert!(
        h.contains("<input type=\"checkbox\" disabled checked> done"),
        "{h}"
    );
}

#[test]
fn quote_and_callouts() {
    assert_eq!(html("> q"), "<blockquote>\n<p>q</p>\n</blockquote>\n");
    let h = html("> [!warning] Careful\n> text");
    assert!(h.starts_with("<aside class=\"callout callout-warning\" role=\"note\">\n<p class=\"callout-title\">Careful</p>"), "{h}");
    let h = html("> [!note]+ Open\n> text\n\n> [!note]- Shut\n> text");
    assert!(
        h.contains("<details class=\"callout callout-note\" open>\n<summary>Open</summary>"),
        "{h}"
    );
    assert!(
        h.contains("<details class=\"callout callout-note\">\n<summary>Shut</summary>"),
        "{h}"
    );
}

#[test]
fn table_with_alignment_classes_and_marks() {
    let h = html("| a | b | c |\n|:--|:-:|--:|\n| **1** | 2 | 3 |");
    assert!(h.contains("<th scope=\"col\" class=\"al-l\">a</th><th scope=\"col\" class=\"al-c\">b</th><th scope=\"col\" class=\"al-r\">c</th>"), "{h}");
    assert!(
        h.contains("<td class=\"al-l\"><strong>1</strong></td>"),
        "{h}"
    );
    assert!(!h.contains("style="), "no style attribute: {h}");
}

#[test]
fn footnotes_are_numbered_by_first_reference_and_linked_back() {
    let md = "x[^b] y[^a] z[^b]\n\n[^a]: Alpha\n[^b]: Beta";
    let h = html(md);
    assert!(h.contains("<a id=\"fnref-1\" href=\"#fn-1\">1</a>"), "{h}");
    assert!(
        h.contains("<a id=\"fnref-1-2\" href=\"#fn-1\">1</a>"),
        "{h}"
    );
    assert!(h.contains("<a id=\"fnref-2\" href=\"#fn-2\">2</a>"), "{h}");
    let beta = h.find("<li id=\"fn-1\">Beta").expect("Beta is note 1");
    assert!(beta < h.find("<li id=\"fn-2\">Alpha").expect("Alpha is note 2"));
}

#[test]
fn footnote_problems_are_reported() {
    use fs_text::{Block, Mark, MarkKind, MarkValue};
    let mut b = Block::paragraph("a");
    b.marks.add(Mark::point(
        MarkKind::FootnoteRef,
        1,
        MarkValue::Name("x".into()),
    ));
    let p = HtmlExporter::default().render_blocks(&[b]);
    assert!(p
        .report
        .problems
        .contains(&Problem::FootnoteUndefined { label: "x".into() }));
    assert!(
        p.body.contains("<sup class=\"fnref\">?</sup>"),
        "{}",
        p.body
    );
    let p = page("a\n\n[^y]: unused");
    assert!(
        p.report
            .problems
            .contains(&Problem::FootnoteUnused { label: "y".into() }),
        "{:?}",
        p.report
    );
    assert!(!p.body.contains("fn-"), "{}", p.body);
}

#[test]
fn front_matter_gives_title_and_description_and_is_not_written() {
    let p = page("---\ntitle: \"My page\"\ndescription: Short\nlang: de\n---\n# Other\n");
    assert_eq!(p.title.as_deref(), Some("My page"));
    assert_eq!(p.description.as_deref(), Some("Short"));
    assert_eq!(p.lang.as_deref(), Some("de"));
    assert!(!p.body.contains("My page"));
    let p = page("# From heading\n\n## Two");
    assert_eq!(p.title.as_deref(), Some("From heading"));
    assert_eq!((p.toc.len(), p.top_headings), (2, 1));
}

#[test]
fn heading_problems_are_reported() {
    let p = page("# a\n\n#### b\n\n# c");
    assert!(p
        .report
        .problems
        .contains(&Problem::HeadingJump { from: 1, to: 4 }));
    assert!(p.report.problems.contains(&Problem::ManyTopHeadings));
}

#[test]
fn images_need_alt_text_and_stay_local() {
    use fs_text::{Block, LinkStyle, LinkValue, Mark, MarkKind, MarkValue};
    let img = |b: &mut Block, a: usize, e: usize, path: &str| {
        let v = MarkValue::Link(LinkValue::new(path, LinkStyle::Inline));
        b.marks.add(Mark::new(MarkKind::Image, a, e, v));
    };
    let mut b = Block::paragraph("ok x");
    img(&mut b, 0, 2, "b.png");
    img(&mut b, 3, 3, "a.png"); // no alt text
    img(&mut b, 4, 4, "https://e.org/c.png");
    let p = HtmlExporter::default().render_blocks(&[b]);
    assert!(
        p.report.problems.contains(&Problem::MissingAlt {
            src: "a.png".into()
        }),
        "{:?}",
        p.report
    );
    assert!(p.report.problems.contains(&Problem::UnsafeMedia {
        src: "https://e.org/c.png".into()
    }));
    assert_eq!(p.media, ["b.png", "a.png"]);
    assert!(
        p.body
            .contains("<img src=\"b.png\" alt=\"ok\" loading=\"lazy\">"),
        "{}",
        p.body
    );
    assert!(!p.body.contains("e.org"), "{}", p.body);
    let md = page("![ok](b.png)");
    assert!(md.report.is_clean());
}

#[test]
fn hard_break_and_soft_break() {
    assert_eq!(html("a  \nb\nc"), "<p>a<br>b\nc</p>\n");
}

#[test]
fn layer_marks_are_not_exported() {
    use fs_text::{Block, BlockKind, Mark, MarkKind, MarkValue};
    let mut b = Block::new(BlockKind::Paragraph, "keep this");
    b.marks.add(Mark::new(
        MarkKind::Passage,
        0,
        4,
        MarkValue::Name("p".into()),
    ));
    b.marks.add(Mark::new(
        MarkKind::Explanation,
        5,
        9,
        MarkValue::Explanation {
            replacement: "other".into(),
        },
    ));
    b.marks.add(Mark::point(
        MarkKind::Variable,
        4,
        MarkValue::Variable {
            name: "who".into(),
            ty: None,
        },
    ));
    let p = HtmlExporter::default().render_blocks(&[b]);
    assert_eq!(p.body, "<p>keep this</p>\n");
    assert!(p
        .report
        .problems
        .contains(&Problem::UnfilledVariable { name: "who".into() }));
}

#[test]
fn exporter_trait_matches_render() {
    let d = import_markdown("# T\n\ntext");
    let e = HtmlExporter::default();
    assert_eq!(e.export(&d), e.render(&d).body);
    assert_eq!(e.export_blocks(&d.blocks), e.export(&d));
}
