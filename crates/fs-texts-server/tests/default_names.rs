//! The default socket and data folder of the server: socket `texts.sock` (never `wiki.sock`) and data folder `wiki` (kept for now, the user's data).
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
    assert_eq!(options.socket, run.join("freesynergy/texts.sock"));
    assert_eq!(options.data, share.join("freesynergy/wiki"));
    // A given folder is taken as it is.
    let given = fs_texts_server::parse_options(["--data", "/elsewhere"].map(Into::into))
        .unwrap()
        .unwrap();
    assert_eq!(given.data, Path::new("/elsewhere"));
    assert!(fs_texts_server::HELP.contains("freesynergy/texts.sock"));
    assert!(fs_texts_server::HELP.contains("freesynergy/wiki (for now),"));
    assert!(!fs_texts_server::HELP.contains("wiki.sock"));
}
