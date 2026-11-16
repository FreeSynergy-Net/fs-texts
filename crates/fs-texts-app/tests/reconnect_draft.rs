//! "Reconnect" ends the window and starts the program again: a text being edited is backed up first (and waited for); when it
//! cannot be, the person is asked; a second click does not open a second window (review of 2026-11-16).
//!
//! Pattern: Observation of the window model, headless, over a lost server (a probe that always fails) and a counting or a
//! failing draft store; the program's restart is a counter.

#[path = "common/mod.rs"]
mod common;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fs_components_workbenches::{Draft, DraftStore, Reader, ReaderWidget, SourceError};
use fs_texts_app::WikiSource;
use fs_texts_app::frontend::Frontend;
use fs_texts_app::frontend_window::{Setup, window_linked};
use fs_texts_app::source::PageMaker;
use fs_texts_app::ui_texts::UiTexts;
use fs_texts_app::window_link::{Link, Probe};
use fs_texts_app::window_modal::ModalSlot;

/// A store that counts its writes, or refuses them.
struct Store {
    fail: bool,
    saved: Mutex<Vec<String>>,
}

impl DraftStore for Store {
    fn save(&self, draft: &Draft) -> Result<(), SourceError> {
        if self.fail {
            return Err(SourceError::new(
                fs_components_workbenches::ErrorKind::Failed,
                "disk full",
            ));
        }
        self.saved.lock().unwrap().push(draft.text.clone());
        Ok(())
    }
    fn load(&self, _: &str) -> Result<Option<Draft>, SourceError> {
        Ok(None)
    }
    fn clear(&self, _: &str) -> Result<(), SourceError> {
        Ok(())
    }
}

fn lost_window(fail: bool, off: bool) -> (Arc<ModalSlot>, Arc<Store>, Arc<AtomicUsize>) {
    let (rt, fixture) = common::wiki();
    let wiki = fixture.wiki().texts();
    rt.block_on(wiki.create("notes.md", "# Notes\n\nA note.\n"))
        .unwrap();
    let source = Arc::new(WikiSource::new(wiki, rt.handle().clone(), vec![]));
    let maker: Arc<dyn PageMaker> = source.clone();
    let store = Arc::new(Store {
        fail,
        saved: Mutex::default(),
    });
    let mut reader = ReaderWidget::new(source, UiTexts::new(false).reader_labels(), None);
    if !off {
        reader = reader.drafts(store.clone());
    }
    reader.with_reader(Reader::refresh_entries);
    reader.with_reader(|r| {
        r.open("notes.md");
        r.begin_edit();
        r.set_draft("# Notes\n\nA typed sentence.\n");
    });
    let setup = Setup {
        labels: UiTexts::new(false).wiki_labels(),
        german: false,
        bare: Arc::new(AtomicBool::new(false)),
        settings: None,
        pages: Some(maker),
    };
    let restarts = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&restarts);
    let probe: Probe = Arc::new(|| Err("gone".into()));
    let link = Link {
        probe,
        reconnect: Arc::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
        socket: PathBuf::from("/run/user/1000/freesynergy/texts.sock"),
        facet: None,
        notes: vec![],
        every: Duration::from_millis(40),
    };
    let (app, modal) =
        window_linked("Texts", reader, Frontend::Confluence, setup, Some(link)).unwrap();
    std::mem::forget((rt, fixture, app));
    let end = Instant::now() + Duration::from_secs(20);
    while !modal.is_open() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(30));
    }
    assert!(modal.is_open(), "the lost-server modal is open");
    (modal, store, restarts)
}

/// One test, in this order: the flag "drafts are not backed up" of the program is process-wide and is set last.
#[test]
fn reconnect_backs_the_draft_up_first_asks_when_it_cannot_and_opens_one_window_only() {
    // A draft that is backed up: written before the restart; a second click does nothing.
    let (modal, store, restarts) = lost_window(false, false);
    assert!(modal.yes());
    assert!(
        store
            .saved
            .lock()
            .unwrap()
            .iter()
            .any(|t| t.contains("A typed sentence")),
        "the text was written to the draft store before the restart"
    );
    assert_eq!(restarts.load(Ordering::SeqCst), 1);
    modal.yes();
    assert_eq!(restarts.load(Ordering::SeqCst), 1, "no second window");
    // A store that refuses, and no store at all (the start could not make one): asked first.
    for (fail, off) in [(true, false), (false, true)] {
        if off {
            fs_texts_app::mode_link::drafts_failed("no drafts".into());
        }
        let (modal, _store, restarts) = lost_window(fail, off);
        assert!(modal.yes());
        assert_eq!(
            restarts.load(Ordering::SeqCst),
            0,
            "asked first (fail={fail}, off={off})"
        );
        assert!(modal.is_open(), "the question is open");
        assert!(modal.yes(), "open anyway");
        assert_eq!(restarts.load(Ordering::SeqCst), 1);
    }
}
