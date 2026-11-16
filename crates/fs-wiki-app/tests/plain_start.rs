//! The plain start (no arguments, as the desktop file runs it): pages can be made; when nothing can be written the window
//! says what is missing and what can be done (rule: never silently grey, never silently read only).
//!
//! Pattern: Integration test over the real wiki (server over a socket), the decision of the start (`start_lock`) and the
//! window model: the menu Page and the status line are read headless, no window is opened.

#[path = "common/mod.rs"]
mod common;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use fs_components_app::{AppWidget, Status};
use fs_components_workbenches::{Reader, ReaderWidget};
use fs_wiki_app::WikiSource;
use fs_wiki_app::args::parse;
use fs_wiki_app::frontend::Frontend;
use fs_wiki_app::frontend_window::{PAGE_NEW, Setup, window_with_modal};
use fs_wiki_app::source::PageMaker;
use fs_wiki_app::start_access::start_lock;

/// The window of a start: the source is made as `mode_read` makes it from the decision of `start_lock`.
fn window_of(words: &[&str], data: &Path) -> AppWidget {
    let args = parse(&words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>()).unwrap();
    let (rt, fixture) = common::wiki();
    let mut source = WikiSource::new(fixture.wiki().texts(), rt.handle().clone(), vec![]);
    if let Some(why) = start_lock(&args, data, false) {
        source = source.read_only_because(why);
    }
    let source = Arc::new(source);
    let maker: Arc<dyn PageMaker> = source.clone();
    let reader = ReaderWidget::new(
        source,
        fs_wiki_app::ui_texts::UiTexts::new(false).reader_labels(),
        None,
    );
    reader.with_reader(Reader::refresh_entries);
    let setup = Setup {
        labels: fs_wiki_app::ui_texts::UiTexts::new(false).wiki_labels(),
        german: false,
        bare: Arc::new(AtomicBool::new(false)),
        settings: None,
        pages: Some(maker),
    };
    let (app, _modal) = window_with_modal("Wiki", reader, Frontend::Confluence, setup).unwrap();
    // The server and the runtime live as long as the window of the test.
    std::mem::forget((rt, fixture));
    app
}

/// (id, label, disabled) of the entries of the menu Page.
fn page_menu(app: &AppWidget) -> Vec<(String, String, bool)> {
    app.with_shell(|s| {
        s.menus()
            .iter()
            .find(|m| m.label == "Page")
            .map(|m| {
                m.entries
                    .iter()
                    .map(|e| (e.id.clone(), e.label.clone(), e.disabled))
                    .collect()
            })
            .unwrap_or_default()
    })
}

#[test]
fn the_plain_start_with_a_writable_data_folder_can_make_pages_and_says_nothing_is_locked() {
    let data = tempfile::tempdir().unwrap();
    let args = parse(&[]).unwrap();
    assert_eq!(start_lock(&args, data.path(), false), None);
    let app = window_of(&[], data.path());
    let menu = page_menu(&app);
    let new = menu.iter().find(|(id, ..)| id == PAGE_NEW).unwrap();
    assert!(!new.2, "New page is active: {new:?}");
    assert_eq!(new.1, "New page");
    assert!(app.with_shell(|s| s.status().cloned()).is_none());
}

#[test]
fn a_data_folder_that_cannot_be_written_says_what_is_missing_and_what_to_do() {
    let dir = tempfile::tempdir().unwrap();
    // A file where the folder should be: nothing can be made below it.
    let file = dir.path().join("not-a-folder");
    std::fs::write(&file, "x").unwrap();
    let data = file.join("fs-wiki");
    let app = window_of(&[], &data);
    let path = data.display().to_string();
    let menu = page_menu(&app);
    assert_eq!(menu.len(), 5);
    for (_, label, disabled) in &menu {
        assert!(*disabled, "{label}");
        assert!(label.contains(&path), "names what is missing: {label}");
        assert!(
            label.contains("check the permissions"),
            "says what to do: {label}"
        );
    }
    assert!(matches!(
        app.with_shell(|s| s.status().cloned()),
        Some(Status::Problem(t)) if t.contains(&path) && t.contains("XDG_DATA_HOME")
    ));
}

#[test]
fn a_start_that_asks_for_reading_only_is_locked_with_the_way_out_and_edit_unlocks_sources() {
    let data = tempfile::tempdir().unwrap();
    let read = parse(&["--read-only".to_owned()]).unwrap();
    let why = start_lock(&read, data.path(), false).unwrap();
    assert!(
        why.contains("--read-only") && why.contains("--edit"),
        "{why}"
    );
    let words: Vec<String> = ["--read", "--source", "docs=."].map(str::to_owned).to_vec();
    let sources = parse(&words).unwrap();
    assert!(
        start_lock(&sources, data.path(), false)
            .unwrap()
            .contains("--edit")
    );
    let editing = parse(&[words, vec!["--edit".to_owned()]].concat()).unwrap();
    assert_eq!(start_lock(&editing, data.path(), false), None);
    assert!(
        start_lock(&read, data.path(), true)
            .unwrap()
            .contains("nur lesen")
    );
}

#[test]
fn a_start_with_the_facet_link_or_as_the_store_starts_it_can_make_pages() {
    // The store starts a program with no argument; the facet link `fs-wiki` adds `--facet wiki`: both are normal starts.
    let data = tempfile::tempdir().unwrap();
    for words in [vec![], vec!["--facet", "wiki"]] {
        let args = parse(&words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>()).unwrap();
        assert_eq!(start_lock(&args, data.path(), false), None, "{words:?}");
        let menu = page_menu(&window_of(&words, data.path()));
        let new = menu.iter().find(|(id, ..)| id == PAGE_NEW).unwrap();
        assert!(!new.2, "New page is active for {words:?}: {new:?}");
    }
}
