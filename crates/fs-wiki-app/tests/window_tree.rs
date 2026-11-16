//! The page tree of the window holds the page that is open, and marks it (finding of 2026-11-14: the pictures of a freshly
//! made page showed "3 pages" and the page was not in the tree).
//!
//! Cause: the tree is built from the list of the source when the window lays the reader out; a list read again by the reader
//! alone (`refresh_entries`) did not reach it. The window now notices a changed list itself and makes the tree again.
//!
//! Pattern: Integration test on the layout of the real window over the real wiki (rows have the id `fs-wk-row-<path>`).

#![cfg(target_os = "linux")]

mod page_rig;

use fs_components::blinc::layout::prelude::RenderTree;
use fs_components_workbenches::Reader;
use fs_wiki_app::frontend::Frontend;
use fs_wiki_app::frontend_window::PAGE_NEW;
use page_rig::{rig, rig_in};

/// The fill of the row of `path` (the mark of the page that is open is a fill).
fn fill_of(tree: &RenderTree, path: &str) -> Option<[f32; 4]> {
    let node = tree.query_by_id(&format!("fs-wk-row-{path}"))?;
    let rn = tree.get_render_node(node)?;
    Some(match rn.props.background.as_ref()? {
        fs_components::blinc::core::Brush::Solid(c) => [c.r, c.g, c.b, c.a],
        _ => [1.0; 4],
    })
}

#[test]
fn a_page_made_in_the_window_is_in_the_tree_and_marked() {
    let _g = page_rig::serial();
    for frontend in [Frontend::Confluence, Frontend::Notion] {
        let r = rig_in(true, frontend);
        r.warm_up();
        assert!(r.app.choose(PAGE_NEW));
        r.modal.submit("Aufgaben");
        assert!(r.modal.field_text().is_none(), "the dialog closed");
        assert!(r.tree_gets("aufgaben.md"), "{frontend:?}: in the tree");
        let tree = r.window_layout(1000.0, 700.0);
        // The mark is a fill: the new page has one, the others have none or another one (not only "they differ").
        let mark = fill_of(&tree, "aufgaben.md");
        assert!(mark.is_some(), "{frontend:?}: the open page has the mark");
        for other in ["notes.md", "handbook.md"] {
            assert_ne!(
                mark,
                fill_of(&tree, other),
                "{frontend:?}: {other} is not marked"
            );
        }
        if frontend == Frontend::Notion {
            // The picture of the window after the dialog: the new page stands in the tree and is marked.
            let dir = std::env::var_os("FS_WIKI_SHOTS").map_or_else(std::env::temp_dir, Into::into);
            std::fs::create_dir_all(&dir).unwrap();
            r.window_picture(1000, 700, &dir.join("seite-ueber-dialog-im-baum.png"))
                .unwrap();
        }
    }
}

#[test]
fn a_page_that_appeared_in_the_list_is_in_the_tree_too() {
    // The rig makes the page in the wiki and tells the reader (as the import does): no dialog of the window is used.
    let _g = page_rig::serial();
    let r = rig(true);
    r.warm_up();
    r.make("tabelle.md", "# Tabelle\n");
    r.open("tabelle.md");
    assert!(r.tree_gets("tabelle.md"));
}

#[test]
fn counter_proof_a_list_changed_behind_the_window_is_not_in_the_tree() {
    // Without the window noticing: the list of the reader is not changed, so the tree cannot show the page either.
    let _g = page_rig::serial();
    let r = rig(true);
    r.warm_up();
    r.rt.block_on(r.wiki.create("lost.md", "# Lost\n")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(700));
    assert!(!r.tree_has("lost.md"));
    r.reader.with_reader(Reader::refresh_entries);
    assert!(
        r.tree_gets("lost.md"),
        "the window follows the reader's list"
    );
}
