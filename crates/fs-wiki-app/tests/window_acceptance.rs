//! Acceptance U1/U2 (boxes) of the user test of fs-wiki 0.0.12 (2026-11-14), in the real wiki window (Notion frontend, window styles)
//! over a real wiki with the user's page. Every point has its measurement on the laid-out window and its picture (kept in the
//! folder named by `FS_WIKI_SHOTS`); the pictures are looked at, the measurements are the counter-proof.
//!
//! Pattern: Observation on the laid-out window and on its picture (`Rig::window_layout`, `Rig::window_picture`).
#![cfg(target_os = "linux")]

mod acceptance;
mod page_rig;

use acceptance::{bounds, click, opened, shots};
use page_rig::serial;

#[test]
fn u1_u2_boxes_are_tall_as_the_text_sit_on_the_line_and_tick_both_ways() {
    let _one = serial();
    let (dir, _keep) = shots();
    let r = opened();
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
