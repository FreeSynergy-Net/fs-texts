//! The first page of a new wiki, as a person does it in the window: an empty database shows "No pages yet" with a button, a
//! click on the button asks for a name, the name makes the page, the text is typed and saved, the page stands in the tree,
//! and after a restart (a new window over the same wiki) it is still there. Pictures: `erste-seite-*.png` in the folder named
//! by `FS_TEXTS_SHOTS`. (User finding 2026-11-16: empty tree, empty page, no word on what to do.)
//!
//! Pattern: Observation on the laid-out window with real mouse events through the router (`click`), and on its picture.
#![cfg(target_os = "linux")]

mod acceptance;
mod page_rig;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use acceptance::{bounds, click, shots};
use fs_components_workbenches::{Reader, ReaderWidget};
use fs_texts_app::WikiSource;
use fs_texts_app::frontend::Frontend;
use fs_texts_app::frontend_window::{Setup, window_with_modal};
use fs_texts_app::source::PageMaker;
use fs_texts_app::ui_texts::UiTexts;
use page_rig::{Rig, rig_with, serial};

const TEXT: &str = "# Erste Seite\n\nDas ist der erste Text.\n";

#[test]
fn an_empty_wiki_says_so_and_the_first_page_can_be_made_and_saved() {
    let _one = serial();
    let (dir, _keep) = shots();
    for (frontend, short) in [
        (Frontend::Notion, "notion"),
        (Frontend::Confluence, "confluence"),
    ] {
        let mut r = rig_with(true, frontend, &[]);
        r.warm_up();
        assert!(r.paths().is_empty(), "the wiki starts empty");
        let (w, h) = (1100.0, 700.0);
        let tree = r.window_layout(w, h);
        r.window_picture(
            1100,
            700,
            &dir.join(format!("erste-seite-{short}-0-probe.png")),
        )
        .unwrap();
        // (1) The empty state: a sentence and a button in the page, the same in the place of the tree.
        for id in [
            "fs-texts-empty-hint",
            "fs-texts-empty-title",
            "fs-texts-empty-page",
            "fs-texts-empty-side",
            "fs-texts-empty-new-page",
            "fs-texts-empty-new-side",
        ] {
            assert!(tree.query_by_id(id).is_some(), "{short}: no element {id}");
        }
        // Readable under an open menu: a menu drops from the bar over the top of the page (five entries, about 230 px).
        let (_, hint_y, _, _) = bounds(&tree, "fs-texts-empty-hint");
        assert!(
            hint_y > 250.0,
            "{short}: the sentence lies low enough, y={hint_y}"
        );
        r.window_picture(
            1100,
            700,
            &dir.join(format!("erste-seite-{short}-1-leer.png")),
        )
        .unwrap();
        // (2) A click on the button asks for the name.
        let mut tree = r.window_layout(w, h);
        click(&mut tree, "fs-texts-empty-new-page", (w, h));
        assert!(
            r.modal.submit("Erste Seite"),
            "{short}: the click asked for a name"
        );
        r.window_picture(
            1100,
            700,
            &dir.join(format!("erste-seite-{short}-2-angelegt.png")),
        )
        .unwrap();
        // (3) Type and save.
        assert!(
            r.reader.with_reader(|x| x.editing().is_some()),
            "{short}: in the editor"
        );
        r.reader.with_reader(|x| x.set_draft(TEXT));
        r.reader.with_reader(Reader::save);
        assert_eq!(r.paths(), vec!["erste-seite.md".to_owned()], "{short}");
        assert!(
            r.text("erste-seite.md").contains("Das ist der erste Text."),
            "{short}: stored"
        );
        assert!(
            r.tree_gets("erste-seite.md"),
            "{short}: the page stands in the tree"
        );
        // The status line no longer says "Page made ... then save": it says it is saved (review of 0.0.13, condition A).
        assert!(
            saved_said(&r),
            "{short}: the status line says Gespeichert, it says {:?}",
            r.status()
        );
        r.window_picture(
            1100,
            700,
            &dir.join(format!("erste-seite-{short}-3-gespeichert.png")),
        )
        .unwrap();
        // (4) Restart: a new window over the same wiki shows the page, not the empty state.
        restart(&mut r, frontend);
        let again = &r;
        again.open("erste-seite.md");
        assert!(
            again.tree_gets("erste-seite.md"),
            "{short}: back after the restart"
        );
        assert!(
            again
                .window_layout(w, h)
                .query_by_id("fs-texts-empty-hint")
                .is_none(),
            "{short}: no empty state any more"
        );
        again
            .window_picture(
                1100,
                700,
                &dir.join(format!("erste-seite-{short}-4-neustart.png")),
            )
            .unwrap();
    }
}

/// Wait (at most five seconds) until the status line says "Gespeichert." (the window sets it in a thread after the save).
fn saved_said(r: &Rig) -> bool {
    let began = std::time::Instant::now();
    while began.elapsed() < std::time::Duration::from_secs(5) {
        if matches!(r.status(), Some(fs_components_app::Status::Info(t)) if t == "Gespeichert.") {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

/// A new window, reader and source over the wiki of `r` (what a restart of the program does).
fn restart(r: &mut Rig, frontend: Frontend) {
    let source = Arc::new(WikiSource::new(
        Arc::clone(&r.wiki),
        r.rt.handle().clone(),
        vec![],
    ));
    let maker: Arc<dyn PageMaker> = source.clone();
    let t = UiTexts::new(true);
    let reader = ReaderWidget::new(source, t.reader_labels(), None);
    reader.with_reader(Reader::refresh_entries);
    let setup = Setup {
        labels: t.wiki_labels(),
        german: true,
        bare: Arc::new(AtomicBool::new(false)),
        settings: None,
        pages: Some(maker),
    };
    let (app, modal) = window_with_modal("Wiki", reader.clone(), frontend, setup).unwrap();
    (r.app, r.reader, r.modal) = (app, reader, modal);
}
