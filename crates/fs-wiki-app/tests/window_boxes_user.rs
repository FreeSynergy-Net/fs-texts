//! The user's page of 2026-11-14 in the real window: every box can be ticked and unticked again, in every frontend.
#![cfg(target_os = "linux")]

mod page_rig;

use fs_components::blinc::layout::event_router::{EventRouter, MouseButton};
use fs_components::blinc::layout::prelude::RenderTree;
use fs_wiki_app::frontend::Frontend;
use page_rig::rig_in;

const PAGE: &str = "# Das ist eine Testseite.\n\ndas ist dann der Test\n\n## mal sehen\n\n| Name | Test | Beschreibung |\n| --- | --- | :---: |\n| Kal | - [ ] Test? | Mal sehen |\n| öl | ölasdkjfö | fkh |\n\n~~Test~~ und _underscore_\n\n- [ ] Das ist ein Test\n- [ ] Das ist ja cool\n\n> das ist ein Test\n\n```\ncool\n```\n\n---\n";

fn click(tree: &mut RenderTree, id: &str) {
    let node = tree
        .query_by_id(id)
        .unwrap_or_else(|| panic!("no element {id}"));
    let b = tree.get_absolute_bounds(node).expect("bounds");
    let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
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
    tree.compute_layout(1400.0, 800.0);
}

#[test]
fn every_box_of_the_user_page_ticks_and_unticks_in_every_frontend() {
    for frontend in [Frontend::Confluence, Frontend::Notion] {
        let r = rig_in(true, frontend);
        r.make("test.md", PAGE);
        r.open("test.md");
        r.warm_up();
        fs_components::blinc::core::reactive::set_stateful_deps_notifier(|ids| {
            fs_components::blinc::layout::check_stateful_deps(ids);
        });
        let mut tree = RenderTree::from_element(&r.app.build(1400.0, 800.0));
        tree.compute_layout(1400.0, 800.0);
        for (id, replace) in [
            ("fs-tv-task-0", ("- [ ] Test?", "- [x] Test?")),
            (
                "fs-tv-task-1",
                ("- [ ] Das ist ein Test", "- [x] Das ist ein Test"),
            ),
            (
                "fs-tv-task-2",
                ("- [ ] Das ist ja cool", "- [x] Das ist ja cool"),
            ),
        ] {
            let before = r.text("test.md");
            click(&mut tree, id);
            let ticked = r.text("test.md");
            assert_eq!(
                ticked,
                before.replace(replace.0, replace.1),
                "{frontend:?}: tick {id}"
            );
            click(&mut tree, id);
            assert_eq!(r.text("test.md"), before, "{frontend:?}: untick {id}");
        }
    }
}
