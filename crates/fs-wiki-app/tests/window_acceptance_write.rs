//! Acceptance U3 (writing surface) of the user test of fs-wiki 0.0.12 (2026-11-14), in the real wiki window (Notion frontend, window styles)
//! over a real wiki with the user's page. Every point has its measurement on the laid-out window and its picture (kept in the
//! folder named by `FS_WIKI_SHOTS`); the pictures are looked at, the measurements are the counter-proof.
//!
//! Pattern: Observation on the laid-out window and on its picture (`Rig::window_layout`, `Rig::window_picture`).
#![cfg(target_os = "linux")]

mod acceptance;
mod page_rig;

use acceptance::{opened, shots};
use page_rig::serial;

#[test]
fn u3_one_writing_surface_enter_makes_a_block_and_nothing_vanishes() {
    let _one = serial();
    let (dir, _keep) = shots();
    let r = opened();
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
