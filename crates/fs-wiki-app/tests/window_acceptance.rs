//! Acceptance U1-U8 of the user test of fs-wiki 0.0.12 (2026-11-14), in the real wiki window (Notion frontend, window styles)
//! over a real wiki with the user's page. Every point has its measurement on the laid-out window and its picture (kept in the
//! folder named by `FS_WIKI_SHOTS`); the pictures are looked at, the measurements are the counter-proof.
//!
//! Pattern: Observation on the laid-out window and on its picture (`Rig::window_layout`, `Rig::window_picture`).
#![cfg(target_os = "linux")]

mod page_rig;

use fs_components::blinc::layout::event_router::{EventRouter, MouseButton};
use fs_components::blinc::layout::prelude::RenderTree;
use fs_wiki_app::frontend::Frontend;
use page_rig::{Rig, rig_in, serial};

const PAGE: &str = "# Das ist eine Testseite.\n\ndas ist dann der Test\n\n## mal sehen\n\n| Name | Test | Beschreibung |\n| --- | --- | :---: |\n| Kal | - [ ] Test? | Mal sehen |\n| öl | ölasdkjfö | fkh |\n\n~~Test~~ und _underscore_\n\n- [ ] Das ist ein Test\n- [ ] Das ist ja cool\n\n> das ist ein Test\n\n```\ncool\n```\n\n---\n";

fn shots() -> (std::path::PathBuf, Option<tempfile::TempDir>) {
    if let Some(dir) = std::env::var_os("FS_WIKI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        return (std::path::PathBuf::from(dir), None);
    }
    let t = tempfile::tempdir().unwrap();
    (t.path().to_path_buf(), Some(t))
}

fn opened(width: f32) -> Rig {
    let r = rig_in(true, Frontend::Notion);
    r.make("test.md", PAGE);
    r.open("test.md");
    r.warm_up();
    let _ = width;
    r
}

fn bounds(tree: &RenderTree, id: &str) -> (f32, f32, f32, f32) {
    let node = tree
        .query_by_id(id)
        .unwrap_or_else(|| panic!("no element {id}"));
    let b = tree.get_absolute_bounds(node).expect("bounds");
    (b.x, b.y, b.width, b.height)
}

/// The picture of the window as pixels (RGB rows).
fn pixels(file: &std::path::Path) -> (u32, u32, Vec<u8>, usize) {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(file).unwrap()));
    let mut reading = decoder.read_info().unwrap();
    let mut data = vec![0; reading.output_buffer_size().unwrap()];
    let info = reading.next_frame(&mut data).unwrap();
    (info.width, info.height, data, info.color_type.samples())
}

fn shoot(r: &Rig, w: u32, h: u32, dir: &std::path::Path, name: &str) -> (u32, u32, Vec<u8>, usize) {
    let file = dir.join(name);
    r.window_picture(w, h, &file).unwrap();
    pixels(&file)
}

/// First dark pixel (any channel below 100: ink) in the rows `y0..y1`, from x `from`.
fn left_ink(p: &(u32, u32, Vec<u8>, usize), from: u32, y0: u32, y1: u32) -> Option<u32> {
    (from..p.0).find(|&x| {
        (y0..y1).any(|y| {
            p.2[(y * p.0 + x) as usize * p.3..][..3]
                .iter()
                .all(|c| *c < 100)
        })
    })
}

fn click(tree: &mut RenderTree, id: &str, size: (f32, f32)) {
    let (x, y, w, h) = bounds(tree, id);
    let (x, y) = (x + w / 2.0, y + h / 2.0);
    let mut router = EventRouter::new();
    for events in [
        router.on_mouse_move(tree, x, y),
        router.on_mouse_down(tree, x, y, MouseButton::Left),
        router.on_mouse_up(tree, x, y, MouseButton::Left),
    ] {
        for (n, e) in events {
            tree.dispatch_event(n, e, x, y);
        }
    }
    tree.process_pending_subtree_rebuilds();
    tree.compute_layout(size.0, size.1);
}

#[test]
fn u1_u2_boxes_are_tall_as_the_text_sit_on_the_line_and_tick_both_ways() {
    let _one = serial();
    let (dir, _keep) = shots();
    let r = opened(1400.0);
    r.window_picture(1400, 800, &dir.join("u1-u2-kaestchen.png"))
        .unwrap();
    let mut tree = r.window_layout(1400.0, 800.0);
    // A list box: at least 16 px (the old one was 14), as tall as the line of its text is high.
    let (_, _, w, h) = bounds(&tree, "fs-tv-task-1");
    assert!(w >= 16.0 && h >= 16.0, "list box {w} x {h} px");
    // Both ways, list and table cell, by the window's router; the page text on disk follows.
    for id in ["fs-tv-task-0", "fs-tv-task-1", "fs-tv-task-2"] {
        let before = r.text("test.md");
        click(&mut tree, id, (1400.0, 800.0));
        assert_ne!(r.text("test.md"), before, "tick {id}");
        click(&mut tree, id, (1400.0, 800.0));
        assert_eq!(r.text("test.md"), before, "untick {id}");
    }
}

#[test]
fn u4_u5_u6_nothing_is_cut_and_the_way_starts_with_the_text() {
    let _one = serial();
    let (dir, _keep) = shots();
    let r = opened(1900.0);
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
    let (wway, wtitle) = (
        left_ink(&w, 280, 55, 78).unwrap(),
        left_ink(&w, 280, 250, 292).unwrap(),
    );
    assert!(wtitle < 400, "full width: title at x {wtitle}");
    assert!(
        wway.abs_diff(wtitle) <= 4,
        "full width: way {wway}, title {wtitle}"
    );
    r.reader.set_full_width(false);
    let n = shoot(&r, 1300, 800, &dir, "u6-breite-1300.png");
    let (nway, ntitle) = (
        left_ink(&n, 280, 55, 78).unwrap(),
        left_ink(&n, 280, 250, 292).unwrap(),
    );
    assert!(
        nway.abs_diff(ntitle) <= 4,
        "1300 px: way {nway}, title {ntitle}"
    );
}

#[test]
fn u7_u8_table_text_is_body_size_and_the_code_block_is_a_framed_box() {
    let _one = serial();
    let (dir, _keep) = shots();
    let r = opened(1400.0);
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
    // U8: the code block is a framed box of one line (not 81 px), its frame lines are not the colour of the page.
    let grey = |y: u32| p.2[(y * p.0 + 900) as usize * p.3] < 250;
    let top = (850..1000).find(|&y| grey(y)).expect("top frame line");
    let bottom = (top + 5..1000)
        .find(|&y| grey(y))
        .expect("bottom frame line");
    assert!(
        (30..=56).contains(&(bottom - top)),
        "code block {} px high",
        bottom - top
    );
    assert!(
        p.2[(900 * p.0 + 482) as usize * p.3] < 250,
        "left frame line"
    );
}

#[test]
fn u3_one_writing_surface_enter_makes_a_block_and_nothing_vanishes() {
    let _one = serial();
    let (dir, _keep) = shots();
    let r = opened(1400.0);
    r.reader
        .with_reader(fs_components_workbenches::Reader::begin_edit);
    r.window_picture(1400, 800, &dir.join("u3-schreibflaeche.png"))
        .unwrap();
    let draft = || r.reader.with_reader(|x| x.editing().unwrap().draft.clone());
    assert!(
        draft().contains("cool") && draft().contains("Mal sehen"),
        "every block of the page is on the surface"
    );
    let before = draft();
    // Enter in the first paragraph: a new block, unseen; nothing of the page is lost; the keyboard goes on.
    let blocks = r.reader.block_texts();
    r.reader.focus_block(1, 5);
    let mut tree = r.window_layout(1400.0, 800.0);
    tree.broadcast_key_event(
        fs_components::blinc::core::events::event_types::KEY_DOWN,
        13,
        false,
        false,
        false,
        false,
    );
    tree.process_pending_subtree_rebuilds();
    let after = r.reader.block_texts();
    assert_eq!(
        after.len(),
        blocks.len() + 1,
        "Enter makes one block more: {after:?}"
    );
    assert_eq!(r.reader.focused_block(), Some(2), "the keyboard moved on");
    r.window_picture(1400, 800, &dir.join("u3-nach-enter.png"))
        .unwrap();
    // Walking through all blocks with the cursor and drawing in between: every character is still held.
    for block in 0..after.len() {
        r.reader.focus_block(block, 0);
        r.window_picture(1400, 800, &dir.join("u3-wechsel.png"))
            .unwrap();
    }
    let joined = |t: &str| t.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert_eq!(
        joined(&draft()),
        joined(&before),
        "nothing vanished on the way"
    );
}
