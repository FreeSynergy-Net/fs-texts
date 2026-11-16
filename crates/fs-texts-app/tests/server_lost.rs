//! A server that goes away AFTER the start is said in the window: the status line names the socket and the error, the menu
//! Page is locked WITH that reason, a modal offers "Reconnect"; when the server answers again the lock is lifted (rule: a
//! missing data source is never silently greyed out).
//!
//! Pattern: Observation of the window model, headless, over a REAL server that the test ends (and a toggled question for the
//! way back); the facet names the program in the sentences.

#[path = "common/mod.rs"]
mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use fs_components_app::{AppWidget, Status};
use fs_components_workbenches::{Reader, ReaderWidget};
use fs_objects::{Store, TursoStore};
use fs_texts_app::WikiSource;
use fs_texts_app::frontend::Frontend;
use fs_texts_app::frontend_window::{Setup, window_linked};
use fs_texts_app::mode_link::probe_of;
use fs_texts_app::source::PageMaker;
use fs_texts_app::ui_texts::UiTexts;
use fs_texts_app::window_link::{Link, Probe};
use fs_texts_app::window_modal::ModalSlot;
use fs_texts_client::RpcTexts;
use fs_texts_testserver::Served;

const SOCKET: &str = "/run/user/1000/freesynergy/texts.sock";

fn window(
    probe: Probe,
    facet: Option<&str>,
    reconnect: Arc<AtomicUsize>,
) -> (AppWidget, Arc<ModalSlot>) {
    let (rt, fixture) = common::wiki();
    let source = Arc::new(WikiSource::new(
        fixture.wiki().texts(),
        rt.handle().clone(),
        vec![],
    ));
    let maker: Arc<dyn PageMaker> = source.clone();
    let reader = ReaderWidget::new(source, UiTexts::new(false).reader_labels(), None);
    reader.with_reader(Reader::refresh_entries);
    let setup = Setup {
        labels: UiTexts::new(false).wiki_labels(),
        german: false,
        bare: Arc::new(AtomicBool::new(false)),
        settings: None,
        pages: Some(maker),
    };
    let link = Link {
        probe,
        reconnect: Arc::new(move || {
            reconnect.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
        socket: PathBuf::from(SOCKET),
        facet: facet.map(str::to_owned),
        notes: vec![],
        every: Duration::from_millis(40),
    };
    let made = window_linked("Texts", reader, Frontend::Confluence, setup, Some(link)).unwrap();
    std::mem::forget((rt, fixture));
    made
}

fn status(app: &AppWidget) -> Option<Status> {
    app.with_shell(|s| s.status().cloned())
}

fn wait<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> T {
    let end = Instant::now() + Duration::from_secs(20);
    while Instant::now() < end {
        if let Some(v) = f() {
            return v;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    panic!("not seen in time: {what}");
}

fn page_entries(app: &AppWidget) -> Vec<(String, bool)> {
    app.with_shell(|s| {
        s.menus()
            .iter()
            .find(|m| m.label == "Page")
            .map(|m| {
                m.entries
                    .iter()
                    .map(|e| (e.label.clone(), e.disabled))
                    .collect()
            })
            .unwrap_or_default()
    })
}

#[test]
fn a_server_that_ends_after_the_start_is_said_in_the_window_with_the_socket_and_the_error() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store: Arc<dyn Store> =
        rt.block_on(async { Arc::new(TursoStore::open_in_memory().await.unwrap()) });
    let served = Served::over(store);
    let texts = Arc::new(RpcTexts::connect(&served.socket).unwrap());
    let probe = probe_of(texts, rt.handle().clone());
    let reconnects = Arc::new(AtomicUsize::new(0));
    let (app, modal) = window(probe, None, Arc::clone(&reconnects));
    std::thread::sleep(Duration::from_millis(300));
    assert!(!modal.is_open(), "a server that runs says nothing");
    assert!(
        !page_entries(&app)
            .iter()
            .any(|(l, _)| l.contains("not reachable"))
    );
    // The server ends while the window runs.
    drop(served);
    let said = wait("the status line says the server is lost", || {
        match status(&app) {
            Some(Status::Problem(t)) if t.contains("connection to the server lost") => Some(t),
            _ => None,
        }
    });
    assert!(said.starts_with("Texts:"), "names the program: {said}");
    assert!(said.contains(SOCKET), "names the socket: {said}");
    assert!(said.len() > SOCKET.len() + 50, "carries the error: {said}");
    wait("the modal with Reconnect", || modal.is_open().then_some(()));
    wait("the menu Page says why it is locked", || {
        let entries = page_entries(&app);
        (!entries.is_empty()
            && entries
                .iter()
                .all(|(l, d)| *d && l.contains("server not reachable")))
        .then_some(())
    });
    // Reconnect starts the program again and ends this window.
    assert!(modal.yes());
    assert_eq!(reconnects.load(Ordering::SeqCst), 1);
}

#[test]
fn the_facet_names_the_program_in_the_sentence_and_the_way_back_lifts_the_lock() {
    let up = Arc::new(AtomicBool::new(false));
    let asked = Arc::clone(&up);
    let probe: Probe = Arc::new(move || {
        if asked.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err("no answer within 3 s".into())
        }
    });
    let (app, modal) = window(probe, Some("wiki"), Arc::default());
    let said = wait("lost", || match status(&app) {
        Some(Status::Problem(t)) => Some(t),
        _ => None,
    });
    assert!(
        said.starts_with("Wiki:") && said.contains("no answer within 3 s"),
        "{said}"
    );
    wait("modal", || modal.is_open().then_some(()));
    up.store(true, Ordering::SeqCst);
    wait("the lock is lifted and the person is told", || {
        let back = matches!(status(&app), Some(Status::Info(t)) if t.contains("is back"));
        let free = page_entries(&app)
            .iter()
            .any(|(l, d)| l == "New page" && !d);
        (back && free && !modal.is_open()).then_some(())
    });
}

/// Nothing open is said in words (what to do), not left as an empty grey area.
#[test]
fn nothing_open_says_what_to_do() {
    let words = UiTexts::new(false).reader_labels();
    assert!(
        words.nothing_open.contains("Choose a page"),
        "{}",
        words.nothing_open
    );
    assert!(
        UiTexts::new(true)
            .reader_labels()
            .nothing_open
            .contains("Wähle eine Seite")
    );
}

/// The diagnosis names program or facet, paths, rights, the answer of the server, the version and the lease, without a display.
#[test]
fn the_diagnosis_says_where_things_are_and_what_the_server_answers() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("texts.sock");
    let words = [
        "--diagnose",
        "--lang",
        "en",
        "--facet",
        "wiki",
        "--socket",
        socket.to_str().unwrap(),
    ];
    let args = fs_texts_app::args::parse(&words.map(str::to_owned)).unwrap();
    let said = fs_texts_app::diagnose::report(&args);
    for part in [
        "Diagnosis: Wiki (facet: wiki)",
        "Program version: ",
        "Socket: ",
        "texts.sock",
        "Runtime folder: ",
        "writable",
        "Lease: ",
        "not reachable",
        "the diagnosis starts no server",
        "Data folder: ",
    ] {
        assert!(said.contains(part), "{part}: {said}");
    }
    let plain = fs_texts_app::diagnose::report(
        &fs_texts_app::args::parse(&[
            "--lang".into(),
            "de".into(),
            "--socket".into(),
            socket.display().to_string(),
        ])
        .unwrap(),
    );
    assert!(
        plain.contains("Diagnose: Texte (Facette: keine)"),
        "{plain}"
    );
}
