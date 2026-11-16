//! The default socket and data folder of the server: socket `texts.sock` (never `wiki.sock`) and data folder `wiki` (kept for now, the user's data).
//!
//! On Unix the defaults are paths below `$XDG_RUNTIME_DIR` and `$XDG_DATA_HOME`; on Windows (`fs-interfaces`, `rpc/transport/windows`)
//! the socket is the named pipe `\\.\pipe\freesynergy-<SID>-texts` (the SID of the user who runs the test: the test checks the
//! form) and the data folder is `%LOCALAPPDATA%\FreeSynergy\wiki`; the XDG variables mean nothing there.
//!
//! One test in this file: it sets the environment, which no other test in the process may read at the same time.

use std::path::Path;

#[test]
fn the_defaults_are_texts_sock_and_the_wiki_data_folder() {
    let run = Path::new("/run-test");
    let share = Path::new("/share-test");
    std::env::set_var("XDG_RUNTIME_DIR", run);
    std::env::set_var("XDG_DATA_HOME", share);
    let options = fs_texts_server::parse_options(std::iter::empty())
        .unwrap()
        .unwrap();
    check_defaults(&options.socket, &options.data, run, share);
    // A given folder is taken as it is.
    let given = fs_texts_server::parse_options(["--data", "/elsewhere"].map(Into::into))
        .unwrap()
        .unwrap();
    assert_eq!(given.data, Path::new("/elsewhere"));
    assert!(fs_texts_server::HELP.contains("freesynergy/texts.sock"));
    assert!(fs_texts_server::HELP.contains("freesynergy/wiki (for now),"));
    assert!(!fs_texts_server::HELP.contains("wiki.sock"));
}

#[cfg(unix)]
fn check_defaults(socket: &Path, data: &Path, run: &Path, share: &Path) {
    assert_eq!(socket, run.join("freesynergy/texts.sock"));
    assert_eq!(data, share.join("freesynergy/wiki"));
}

#[cfg(windows)]
fn check_defaults(socket: &Path, data: &Path, _run: &Path, _share: &Path) {
    let pipe = socket.to_string_lossy();
    let sid = pipe
        .strip_prefix(r"\\.\pipe\freesynergy-")
        .and_then(|rest| rest.strip_suffix("-texts"))
        .unwrap_or_else(|| {
            panic!("not of the form \\\\.\\pipe\\freesynergy-<SID>-texts: {pipe:?}")
        });
    assert!(
        sid.starts_with("S-1-") && sid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "<SID> is a security identifier (S-1-...): {pipe:?}"
    );
    let local = std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA is set for a user");
    assert_eq!(data, Path::new(&local).join("FreeSynergy").join("wiki"));
}
