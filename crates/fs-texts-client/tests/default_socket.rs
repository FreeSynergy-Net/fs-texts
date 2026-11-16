//! The default socket of the client is the one of the server: `texts.sock`, never `wiki.sock` (regression).
//! On Unix that is a socket file below `$XDG_RUNTIME_DIR`; on Windows the system has no such folder and the server listens on a
//! named pipe `\\.\pipe\freesynergy-<SID>-texts` (`fs-interfaces`, `rpc/transport/windows/name.rs`, `default_name`), `<SID>` being the
//! security identifier of the user who runs the test: the test checks the form, not the value.
//!
//! One test in this file: it sets the environment.

use fs_texts_client::RpcTexts;

#[test]
fn the_default_socket_is_texts_sock() {
    std::env::set_var("XDG_RUNTIME_DIR", "/run-test");
    let socket = RpcTexts::default_socket();
    check(socket.as_deref().and_then(std::path::Path::to_str));
}

#[cfg(unix)]
fn check(socket: Option<&str>) {
    assert_eq!(socket, Some("/run-test/freesynergy/texts.sock"));
}

#[cfg(windows)]
fn check(pipe: Option<&str>) {
    let pipe = pipe.expect("a named pipe for the user");
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
    assert!(!pipe.contains("wiki"), "never the old name: {pipe:?}");
}
