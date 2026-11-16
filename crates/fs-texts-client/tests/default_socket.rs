//! The default socket of the client is the one of the server: `texts.sock`, never `wiki.sock` (regression).
//!
//! One test in this file: it sets the environment.

use std::path::Path;

use fs_texts_client::RpcTexts;

#[test]
fn the_default_socket_is_texts_sock() {
    std::env::set_var("XDG_RUNTIME_DIR", "/run-test");
    assert_eq!(
        RpcTexts::default_socket().as_deref(),
        Some(Path::new("/run-test/freesynergy/texts.sock"))
    );
}
