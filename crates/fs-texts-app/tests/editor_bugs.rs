//! The three findings of the hand test of 2026-11-16 (b1 to b3) on the writing surface of the Notion frontend, each as a
//! measurement on the picture of the real window (title bar, styles of the app) after real key and mouse events through the
//! laid-out tree. The surface is the Blinc text area of `fs-components` (`view/wiki/surface_view.rs`, `view/editor.rs`).
//!
//! b1: after ">" the cursor does not stand right behind the text. b2: a new line makes the block grow too late, the top
//! of the text is gone until the next key. b3: a marking cannot be seen (double click, menu).
//! Findings and causes: `.scratch/wiki-0013-bilder/bugs/README.md`.
//!
//! Pattern: Observation on the picture (pixel probes: ink, caret, marking) of the window the person sees.
#![cfg(target_os = "linux")]

mod acceptance;
mod page_rig;

use acceptance::{pixels, shots};
use fs_components::blinc::core::events::event_types::KEY_DOWN;
use fs_components::blinc::layout::event_router::{EventRouter, MouseButton};
use fs_components::blinc::layout::prelude::RenderTree;
use fs_texts_app::frontend::Frontend;
use page_rig::{Rig, rig_in, serial};

const SIZE: (f32, f32) = (1400.0, 800.0);
type Shot = (u32, u32, Vec<u8>, usize);

/// A Notion window editing a page of two blocks; the first block's text stands at about y = 374, x = 488.
fn editing() -> (Rig, RenderTree) {
    let r = rig_in(true, Frontend::Notion);
    r.make("p.md", "# Titel\n\nerster Absatz hier\n\nzweiter Absatz\n");
    r.open("p.md");
    r.warm_up();
    r.reader
        .with_reader(fs_components_workbenches::Reader::begin_edit);
    let tree = r.window_layout(SIZE.0, SIZE.1);
    (r, tree)
}

fn shoot(r: &Rig, dir: &std::path::Path, name: &str) -> Shot {
    let file = dir.join(name);
    r.window_picture(1400, 800, &file).unwrap();
    pixels(&file)
}

fn at(p: &Shot, x: u32, y: u32) -> [u8; 3] {
    let i = (y * p.0 + x) as usize * p.3;
    [p.2[i], p.2[i + 1], p.2[i + 2]]
}

fn ink(c: [u8; 3]) -> bool {
    c.iter().all(|v| *v < 100)
}

/// Caret blue (light blue, thin) as opposed to the marking (the same hue, wide): a pixel more blue than red.
fn blue(c: [u8; 3]) -> bool {
    c[2] >= 240 && u16::from(c[0]) + 30 <= u16::from(c[2]) && c[0] < 235
}

fn count(
    p: &Shot,
    x: std::ops::Range<u32>,
    y: std::ops::Range<u32>,
    f: fn([u8; 3]) -> bool,
) -> usize {
    x.flat_map(|x| y.clone().map(move |y| (x, y)))
        .filter(|&(x, y)| f(at(p, x, y)))
        .count()
}

fn key(tree: &mut RenderTree, code: u32, shift: bool, ctrl: bool) {
    tree.broadcast_key_event(KEY_DOWN, code, shift, ctrl, false, false);
    tree.process_pending_subtree_rebuilds();
}

fn typed(tree: &mut RenderTree, text: &str) {
    for ch in text.chars() {
        tree.broadcast_text_input_event(ch, false, false, false, false);
    }
    tree.process_pending_subtree_rebuilds();
}

/// The caret (blue) and the last ink of `text` typed into a new block, and the caret of the still empty block.
fn typed_probe(dir: &std::path::Path, n: usize, text: &str) -> (u32, u32, u32) {
    let (r, mut tree) = editing();
    r.reader.focus_block(0, 18);
    key(&mut tree, 13, false, false);
    let empty = shoot(&r, dir, "b1-0-neuer-block.png");
    typed(&mut tree, text);
    let p = shoot(&r, dir, &format!("b1-vorher-{n}.png"));
    let rows = 395..425;
    let first_blue = |p: &Shot| (480..900).find(|&x| rows.clone().any(|y| blue(at(p, x, y))));
    let last_ink = (480..900)
        .rev()
        .find(|&x| rows.clone().any(|y| ink(at(&p, x, y))));
    (
        first_blue(&p).expect(text),
        last_ink.expect(text),
        first_blue(&empty).expect("caret of the empty block"),
    )
}

/// b1 (user: with ">" the cursor stands about 0.5 cm too far in front; suspected: wrong width of the glyph): for several strings typed
/// into a new block the caret stands right behind the last ink. How far "right behind" may be is measured in the width of a
/// character of the renderer at hand, not in fixed pixels: at most three quarters of the advance of an "m" (12 px here, so
/// 9 px). Measured: 2 px locally (3 px for "|" and "<>"), 5 px for "|" on the CI runner (other font rendering, software GPU);
/// the fault of the hand test is about 19 px (a character off). A trailing space has no ink of its own, so the last ink is the
/// one before it: a space is one space wide more, 8 px at most (unchanged).
/// Not red in this rig (see the README): the guard that stays for the spike on inline rendering.
#[test]
fn b1_the_caret_stands_right_behind_what_was_typed() {
    let _one = serial();
    let (dir, _keep) = shots();
    let (m_caret, _, m_start) = typed_probe(&dir, 99, "m");
    let m_width = m_caret - m_start;
    let near = m_width * 3 / 4;
    for (n, text) in [
        ">", "> ", ">>>", "<>", "a>b", "abc", "W>", "m", "#", "|", "-", "iii",
    ]
    .into_iter()
    .enumerate()
    {
        let (caret, last_ink, _) = typed_probe(&dir, n, text);
        let room = if text.ends_with(' ') { 8 } else { near };
        assert!(
            caret + 1 >= last_ink && caret.saturating_sub(last_ink) <= room,
            "{text:?}: the caret (x={caret}) stands right behind the text (last ink x={last_ink}); \
             allowed {room} px (3/4 of the width of an \"m\", {m_width} px)"
        );
    }
}

/// b2: Enter at the end of the page: the block above is still on the picture in the same place (the area grows in the same
/// frame as the new line; the top of the text does not scroll away).
#[test]
#[ignore = "red until fs-components fixes the growing area (FIX-VORSCHLAG b2); run with --ignored"]
fn b2_a_new_line_keeps_the_top_of_the_text_visible() {
    let _one = serial();
    let (dir, _keep) = shots();
    let (r, mut tree) = editing();
    r.reader.focus_block(1, 14); // the end of the text: the cursor is on the new last line
    let before = shoot(&r, &dir, "b2-0-vor-enter.png");
    let (xs, ys) = (480..700, 360..390);
    let ink_before = count(&before, xs.clone(), ys.clone(), ink);
    assert!(
        ink_before > 100,
        "the first block is on the picture: {ink_before}"
    );
    key(&mut tree, 13, false, false);
    let after = shoot(&r, &dir, "b2-vorher.png");
    let ink_after = count(&after, xs, ys, ink);
    assert!(
        ink_after * 10 >= ink_before * 9,
        "after Enter the first block is still there: ink {ink_before} before, {ink_after} after"
    );
}

fn double_click(tree: &mut RenderTree, x: f32, y: f32) {
    let mut router = EventRouter::new();
    for _ in 0..2 {
        for events in [
            router.on_mouse_move(tree, x, y),
            router.on_mouse_down(tree, x, y, MouseButton::Left),
            router.on_mouse_up(tree, x, y, MouseButton::Left),
        ] {
            for (n, e) in events {
                tree.dispatch_event(n, e, x, y);
            }
        }
    }
    tree.process_pending_subtree_rebuilds();
}

/// The marking is on the picture: wide blue-ish ground behind the text of the first block (the caret alone is 2 px wide).
fn marked(p: &Shot) -> usize {
    count(p, 480..800, 362..388, blue)
}

/// Control for b3: a marking made with the keyboard (Shift and arrow keys) IS drawn, so the probe sees a marking.
#[test]
fn b3_control_a_keyboard_marking_is_drawn() {
    let _one = serial();
    let (dir, _keep) = shots();
    let (r, mut tree) = editing();
    r.reader.focus_block(0, 0);
    for _ in 0..6 {
        key(&mut tree, 39, true, false);
    }
    assert_eq!(r.reader.selected_text().as_deref(), Some("erster"));
    let p = shoot(&r, &dir, "b3-kontrolle-tastatur.png");
    assert!(marked(&p) > 150, "marking drawn: {}", marked(&p));
}

/// b3 (double click): a double click on a word marks the word, and the marking is on the picture.
#[test]
#[ignore = "red until Blinc/fs-components mark a word on double click (FIX-VORSCHLAG b3); run with --ignored"]
fn b3_a_double_click_marks_the_word_and_shows_it() {
    let _one = serial();
    let (dir, _keep) = shots();
    let (r, mut tree) = editing();
    r.reader.focus_block(0, 0);
    double_click(&mut tree, 510.0, 374.0);
    let p = shoot(&r, &dir, "b3-doppelklick-vorher.png");
    assert_eq!(
        r.reader.selected_text().as_deref(),
        Some("erster"),
        "the word is marked"
    );
    assert!(marked(&p) > 150, "marking drawn: {}", marked(&p));
}

/// b3 (menu): the entry the block menu has marked stands out from the others (luminance ratio of at least 1.3 against the
/// menu ground; the mark was 1.1, a pale blue nobody sees).
#[test]
#[ignore = "red until fs-components styles the marked entry of the block menu (FIX-VORSCHLAG b3); run with --ignored"]
fn b3_the_marked_entry_of_the_block_menu_stands_out() {
    let _one = serial();
    let (dir, _keep) = shots();
    let (rig, mut tree) = editing();
    rig.reader.focus_block(0, 18);
    key(&mut tree, 13, false, false);
    shoot(&rig, &dir, "b3-menue-0.png");
    typed(&mut tree, "/");
    let laid = rig.window_layout(SIZE.0, SIZE.1);
    let (_, top, _, height) = acceptance::bounds(&laid, "fs-wk-slash-0");
    let shot = shoot(&rig, &dir, "b3-menue-vorher.png");
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (marked_row, plain_row) = ((top + height / 2.0) as u32, (top + height * 1.5) as u32);
    let (on, off) = (
        luminance(at(&shot, 700, marked_row)),
        luminance(at(&shot, 700, plain_row)),
    );
    let ratio = (on.max(off) + 0.05) / (on.min(off) + 0.05);
    assert!(
        ratio >= 1.3,
        "marked entry vs plain entry: ratio {ratio:.2}"
    );
}

fn luminance(c: [u8; 3]) -> f32 {
    let lin = |v: u8| {
        let s = f32::from(v) / 255.0;
        if s <= 0.039_28 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}
