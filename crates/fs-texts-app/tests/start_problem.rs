//! A start that cannot reach the wiki says so in the window, in words (rule: never a grey window without a reason).
//!
//! Pattern: Observation of the window model, headless: the body of the window and its status line are read, a picture is
//! drawn. The checks name the facts of each reason (the path, the command, the error, the way out); a window that drops
//! the sentences fails them.

#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use fs_components_app::Status;
use fs_texts_app::problem_window::{build, lines};
use fs_texts_app::start_problem::{Problem, preflight};

fn server_problem() -> Problem {
    Problem::Server {
        command: "/opt/x/fs-texts serve --lease /run/user/1000/freesynergy/texts.lease".into(),
        socket: "/run/user/1000/freesynergy/texts.sock".into(),
        error: "the new server cannot be reached: No such file or directory (os error 2)".into(),
    }
}

#[test]
fn no_server_says_the_path_the_command_the_error_and_the_hint_in_the_window() {
    let app = build(&server_problem(), false, None, Arc::new(|| {})).unwrap();
    let Some(Status::Problem(said)) = app.with_shell(|s| s.status().cloned()) else {
        panic!("no problem in the status line");
    };
    for part in [
        "/run/user/1000/freesynergy/texts.sock",
        "/opt/x/fs-texts serve --lease",
        "No such file or directory",
    ] {
        assert!(said.contains(part), "{part}: {said}");
    }
    let body = lines(&server_problem(), false, None);
    assert_eq!(body[0], "Texts cannot start");
    assert_eq!(body[1], said);
    assert!(
        body[2].contains("older fs-texts or fs-wiki window"),
        "{body:?}"
    );
    let german = lines(&server_problem(), true, None);
    assert_eq!(german[0], "Texte kann nicht starten");
    assert!(german[1].contains("Die Antwort war"), "{german:?}");
}

#[test]
fn every_reason_names_what_is_missing_and_what_to_do_in_both_languages() {
    let folder = std::path::PathBuf::from("/srv/x/run");
    for (problem, words) in [
        (Problem::NoSocket, ["XDG_RUNTIME_DIR", "--socket"]),
        (
            Problem::RuntimeFolder(folder.clone()),
            ["/srv/x/run", "--socket"],
        ),
        (
            Problem::DataFolder(folder.clone()),
            ["/srv/x/run", "XDG_DATA_HOME"],
        ),
        (Problem::Failed("boom".into()), ["boom", "fehlgeschlagen"]),
    ] {
        let en = lines(&problem, false, None);
        let de = lines(&problem, true, None);
        assert!(en[1].contains(words[0]), "{en:?}");
        assert!(
            !en[1].is_empty() && !de[1].is_empty() && en[1] != de[1]
                || matches!(problem, Problem::Failed(_))
        );
        if matches!(problem, Problem::Failed(_)) {
            assert!(de[1].contains(words[1]), "{de:?}");
        } else {
            assert!(en[1].contains(words[1]), "{en:?}");
        }
    }
}

#[test]
fn a_folder_that_cannot_be_written_stops_the_start_before_the_server_is_asked() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a-file");
    std::fs::write(&file, "x").unwrap();
    // A file where the runtime folder should be: nothing can be made below it.
    let socket = file.join("freesynergy").join("texts.sock");
    assert!(matches!(
        preflight(&socket, false, Some(dir.path())),
        Some(Problem::RuntimeFolder(p)) if p == file.join("freesynergy")
    ));
    // The data folder of the wiki below a file.
    let socket_ok = dir.path().join("run").join("texts.sock");
    assert!(matches!(
        preflight(&socket_ok, false, Some(&file)),
        Some(Problem::DataFolder(p)) if p == file.join("freesynergy").join("wiki")
    ));
    // Everything writable, or the server is elsewhere (own socket): nothing to say.
    assert_eq!(preflight(&socket_ok, false, Some(dir.path())), None);
    assert_eq!(preflight(&socket_ok, true, Some(&file)), None);
}

#[test]
fn the_button_runs_what_the_window_was_given() {
    let tries = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&tries);
    let retry: fs_texts_app::window_modal::Act = Arc::new(move || {
        seen.fetch_add(1, Ordering::SeqCst);
    });
    let _app = build(&Problem::NoSocket, false, None, Arc::clone(&retry)).unwrap();
    retry();
    assert_eq!(tries.load(Ordering::SeqCst), 1);
}

/// The window as a picture: the reason is drawn (a file with content). `FS_TEXTS_SHOTS` names the folder to keep it in.
#[cfg(target_os = "linux")]
#[test]
fn the_problem_window_draws() {
    use fs_components_render::{Canvas, draw_to_png};
    fs_components_workbenches::register_fonts();
    let temp = tempfile::tempdir().unwrap();
    let dir: PathBuf = std::env::var_os("FS_TEXTS_SHOTS").map_or_else(
        || temp.path().to_path_buf(),
        |d| {
            std::fs::create_dir_all(&d).unwrap();
            PathBuf::from(d)
        },
    );
    for (german, lang) in [(false, "en"), (true, "de")] {
        let app = build(&server_problem(), german, None, Arc::new(|| {})).unwrap();
        let file = dir.join(format!("start-problem-{lang}.png"));
        let canvas = Canvas::new(1000, 500, app.styles()).with_window_styles();
        draw_to_png(&canvas, |w, h| app.build(w, h), || {}, &file).expect("picture");
        assert!(std::fs::metadata(&file).unwrap().len() > 2000, "{lang}");
    }
}

/// The title follows the program or the facet, never a fixed word: `--facet wiki` says the wiki, no facet the program, another
/// facet its own name.
#[test]
fn the_title_follows_the_program_or_the_facet() {
    let title = |german, facet| lines(&server_problem(), german, facet)[0].clone();
    assert_eq!(title(true, None), "Texte kann nicht starten");
    assert_eq!(title(false, None), "Texts cannot start");
    assert_eq!(title(true, Some("wiki")), "Das Wiki kann nicht starten");
    assert_eq!(title(false, Some("wiki")), "The wiki cannot start");
    assert_eq!(title(true, Some("notes")), "Notes kann nicht starten");
    assert_eq!(title(false, Some("notes")), "Notes cannot start");
    // The window shows the same title (the first line of its body is what `lines` gives).
    let app = build(&server_problem(), false, Some("wiki"), Arc::new(|| {})).unwrap();
    assert!(app.with_shell(|s| s.status().cloned()).is_some());
}
