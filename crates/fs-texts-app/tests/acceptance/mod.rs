//! What the acceptance tests of the wiki window share: the user's page, the folder of the pictures, measuring on the laid-out
//! window and on its picture.
#![allow(dead_code)]

use super::page_rig::{Rig, rig_in};
use fs_components::blinc::layout::event_router::{EventRouter, MouseButton};
use fs_components::blinc::layout::prelude::RenderTree;
use fs_texts_app::frontend::Frontend;

pub const PAGE: &str = "# Das ist eine Testseite.\n\ndas ist dann der Test\n\n## mal sehen\n\n| Name | Test | Beschreibung |\n| --- | --- | :---: |\n| Kal | - [ ] Test? | Mal sehen |\n| öl | ölasdkjfö | fkh |\n\n~~Test~~ und _underscore_\n\n- [ ] Das ist ein Test\n- [ ] Das ist ja cool\n\n> das ist ein Test\n\n```\ncool\n```\n\n---\n";

pub fn shots() -> (std::path::PathBuf, Option<tempfile::TempDir>) {
    if let Some(dir) = std::env::var_os("FS_TEXTS_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        return (std::path::PathBuf::from(dir), None);
    }
    let t = tempfile::tempdir().unwrap();
    (t.path().to_path_buf(), Some(t))
}

pub fn opened() -> Rig {
    let r = rig_in(true, Frontend::Notion);
    r.make("test.md", PAGE);
    r.open("test.md");
    r.warm_up();
    r
}

pub fn bounds(tree: &RenderTree, id: &str) -> (f32, f32, f32, f32) {
    let node = tree
        .query_by_id(id)
        .unwrap_or_else(|| panic!("no element {id}"));
    let b = tree.get_absolute_bounds(node).expect("bounds");
    (b.x, b.y, b.width, b.height)
}

/// The picture of the window as pixels (RGB rows).
pub fn pixels(file: &std::path::Path) -> (u32, u32, Vec<u8>, usize) {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(file).unwrap()));
    let mut reading = decoder.read_info().unwrap();
    let mut data = vec![0; reading.output_buffer_size().unwrap()];
    let info = reading.next_frame(&mut data).unwrap();
    (info.width, info.height, data, info.color_type.samples())
}

pub fn shoot(
    r: &Rig,
    w: u32,
    h: u32,
    dir: &std::path::Path,
    name: &str,
) -> (u32, u32, Vec<u8>, usize) {
    let file = dir.join(name);
    r.window_picture(w, h, &file).unwrap();
    pixels(&file)
}

/// First dark pixel (any channel below 100: ink) in the rows `y0..y1`, from x `from`.
pub fn left_ink(p: &(u32, u32, Vec<u8>, usize), from: u32, y0: u32, y1: u32) -> Option<u32> {
    (from..p.0).find(|&x| {
        (y0..y1).any(|y| {
            p.2[(y * p.0 + x) as usize * p.3..][..3]
                .iter()
                .all(|c| *c < 100)
        })
    })
}

pub fn click(tree: &mut RenderTree, id: &str, size: (f32, f32)) {
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
