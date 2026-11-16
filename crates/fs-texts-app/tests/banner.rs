//! A wiki that cannot be changed says why in a line above the page, not only in the grey entries of the menu: the layouts of
//! `fs-components` draw no Edit button at all then, so the page alone would show a missing button without a word (the likely
//! finding of the user of 2026-11-16). A wiki that can be changed shows no such line.
//!
//! Pattern: Observation on the laid-out window and on its picture (`Rig::window_layout`, `Rig::window_picture`).
#![cfg(target_os = "linux")]

mod acceptance;
mod page_rig;

use acceptance::{bounds, shots};
use fs_texts_app::frontend::Frontend;
use fs_texts_app::window_link::Link;
use page_rig::{rig_linked, rig_locked, rig_with, serial};

const WHY: &str = "nur lesen: dieses Wiki wird hier nicht bearbeitet";

#[test]
fn a_locked_wiki_says_why_above_the_page_and_an_open_one_does_not() {
    let _one = serial();
    let (dir, _keep) = shots();
    for (frontend, short) in [
        (Frontend::Notion, "notion"),
        (Frontend::Confluence, "confluence"),
    ] {
        let pages = [("notes.md", "# Notes\n\nA note.\n")];
        let locked = rig_locked(true, frontend, &pages, Some(WHY));
        locked.open("notes.md");
        locked.warm_up();
        let tree = locked.window_layout(1100.0, 700.0);
        let (_, _, w, h) = bounds(&tree, "fs-texts-banner");
        assert!(w > 400.0 && h > 20.0, "{short}: the line is drawn: {w}x{h}");
        locked
            .window_picture(1100, 700, &dir.join(format!("gesperrt-{short}-banner.png")))
            .unwrap();
        let open = rig_with(true, frontend, &pages);
        open.open("notes.md");
        open.warm_up();
        assert!(
            open.window_layout(1100.0, 700.0)
                .query_by_id("fs-texts-banner")
                .is_none(),
            "{short}: a wiki that can be changed has no such line"
        );
    }
}

/// The server goes away: the line above the page says "server not reachable" (the wiki itself is not read only: only the loss can make the line),
/// in both frontends, and a server that answers shows no line (review of 0.0.13, condition B).
#[test]
fn a_lost_server_says_so_above_the_page() {
    let _one = serial();
    let (dir, _keep) = shots();
    for (frontend, short) in [
        (Frontend::Notion, "notion"),
        (Frontend::Confluence, "confluence"),
    ] {
        let pages = [("notes.md", "# Notes\n\nA note.\n")];
        let link = |probe: Result<(), String>| Link {
            probe: std::sync::Arc::new(move || probe.clone()),
            reconnect: std::sync::Arc::new(|| Ok(())),
            socket: std::path::PathBuf::from("/run/user/1000/freesynergy/texts.sock"),
            facet: None,
            notes: vec![],
            every: std::time::Duration::from_millis(40),
        };
        let lost = rig_linked(true, frontend, &pages, None, Some(link(Err("weg".into()))));
        lost.open("notes.md");
        lost.warm_up();
        let began = std::time::Instant::now();
        let mut tree = lost.window_layout(1100.0, 700.0);
        while tree.query_by_id("fs-texts-banner").is_none() && began.elapsed().as_secs() < 10 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            tree = lost.window_layout(1100.0, 700.0);
        }
        let (_, _, w, h) = bounds(&tree, "fs-texts-banner");
        assert!(w > 400.0 && h > 20.0, "{short}: the line is drawn: {w}x{h}");
        lost.window_picture(1100, 700, &dir.join(format!("verloren-{short}-banner.png")))
            .unwrap();
        let fine = rig_linked(true, frontend, &pages, None, Some(link(Ok(()))));
        fine.open("notes.md");
        fine.warm_up();
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(
            fine.window_layout(1100.0, 700.0)
                .query_by_id("fs-texts-banner")
                .is_none(),
            "{short}: a server that answers has no such line"
        );
    }
}
