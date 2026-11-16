//! Acceptance U4-U8 (layout) of the user test of fs-texts 0.0.12 (2026-11-14), in the real wiki window (Notion frontend, window styles)
//! over a real wiki with the user's page. Every point has its measurement on the laid-out window and its picture (kept in the
//! folder named by `FS_TEXTS_SHOTS`); the pictures are looked at, the measurements are the counter-proof.
//!
//! Pattern: Observation on the laid-out window and on its picture (`Rig::window_layout`, `Rig::window_picture`).
#![cfg(target_os = "linux")]

mod acceptance;
mod page_rig;

use acceptance::{left_ink, opened, shoot, shots};
use page_rig::serial;

#[test]
fn u4_u5_u6_nothing_is_cut_and_the_way_starts_with_the_text() {
    let _one = serial();
    let (dir, _keep) = shots();
    let r = opened();
    // The way to the page is the line at y 55..78, the title of the page the big letters at y 250..290.
    let p = shoot(&r, 1900, 900, &dir, "u4-u6-breite-1900.png");
    let (way, title) = (
        left_ink(&p, 280, 55, 78).expect("way"),
        left_ink(&p, 280, 250, 292).expect("title"),
    );
    assert!(
        way.abs_diff(title) <= 4,
        "way at x {way}, title at x {title} (1900 px window)"
    );
    assert!(title > 500, "the column is centred: title at x {title}");
    // U4: the page icon is whole under the top line (ink in its rows, none above the line of the head).
    assert!(
        left_ink(&p, 280, 150, 230).is_some(),
        "the page icon is drawn whole"
    );
    // U5: the marks of the outline at the right edge are whole and keep off the edge (ink left of the last 8 px).
    let mark = (280..p.0).rev().find(|&x| {
        (140..190).any(|y| {
            p.2[(y * p.0 + x) as usize * p.3..][..3]
                .iter()
                .all(|c| *c < 200)
        })
    });
    assert!(
        mark.is_some_and(|x| x + 8 <= p.0),
        "outline marks at x {mark:?} of {}",
        p.0
    );
    // Full width: the text begins beside the sidebar again, the way with it.
    r.reader.set_full_width(true);
    let w = shoot(&r, 1900, 900, &dir, "u6-volle-breite-1900.png");
    let (full_way, full_title) = (
        left_ink(&w, 280, 55, 78).unwrap(),
        left_ink(&w, 280, 250, 292).unwrap(),
    );
    assert!(full_title < 400, "full width: title at x {full_title}");
    assert!(
        full_way.abs_diff(full_title) <= 4,
        "full width: way {full_way}, title {full_title}"
    );
    r.reader.set_full_width(false);
    let n = shoot(&r, 1300, 800, &dir, "u6-breite-1300.png");
    let (narrow_way, narrow_title) = (
        left_ink(&n, 280, 55, 78).unwrap(),
        left_ink(&n, 280, 250, 292).unwrap(),
    );
    assert!(
        narrow_way.abs_diff(narrow_title) <= 4,
        "1300 px: way {narrow_way}, title {narrow_title}"
    );
}

#[test]
fn u7_u8_table_text_is_body_size_and_the_code_block_is_a_box_with_a_ground() {
    let _one = serial();
    let (dir, _keep) = shots();
    let r = opened();
    let p = shoot(&r, 1400, 1100, &dir, "u7-u8-tabelle-code.png");
    let ink = |x0: u32, x1: u32, y0: u32, y1: u32| -> (u32, u32) {
        let rows: Vec<u32> = (y0..y1)
            .filter(|&y| {
                (x0..x1).any(|x| {
                    p.2[(y * p.0 + x) as usize * p.3..][..3]
                        .iter()
                        .all(|c| *c < 100)
                })
            })
            .collect();
        (*rows.first().unwrap(), *rows.last().unwrap())
    };
    // U7: "Kal" in a cell and "das" in the paragraph (both a capital or an ascender and no descender) are equally high.
    let (c0, c1) = ink(494, 520, 545, 580);
    let (p0, p1) = ink(482, 512, 345, 380);
    assert!(
        (c1 - c0).abs_diff(p1 - p0) <= 1 && c1 - c0 >= 10,
        "cell text {} px high, paragraph text {} px high",
        c1 - c0,
        p1 - p0
    );
    // U8: the code block is a box of one line (not 81 px) with its own ground (0.0.20: light blue, not the colour of
    // the page): the run of non-white pixels down the column x = 900 is as high as one line plus its padding.
    let grey = |y: u32| p.2[(y * p.0 + 900) as usize * p.3] < 250;
    let top = (850..1000).find(|&y| grey(y)).expect("top of the box");
    let bottom = (top + 5..1000)
        .find(|&y| !grey(y))
        .expect("bottom of the box");
    assert!(
        (30..=56).contains(&(bottom - top)),
        "code block {} px high",
        bottom - top
    );
    assert!(
        p.2[(900 * p.0 + 482) as usize * p.3] < 250,
        "left edge of the box"
    );
}
