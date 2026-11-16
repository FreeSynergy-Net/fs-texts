//! The first start of a user who has nothing yet (finding of the review of 2026-11-16: the start made the data folder with the
//! default rights, 0755, and the server refused it: "must belong to this user alone (0700)"). Nothing is made beforehand
//! here: the data folder and the runtime folder of the program do not exist.
//!
//! Pattern: Example-based tests of the real binary in a temporary HOME (see `workflows.rs`); `--check` is the headless way
//! through the same start as the window (folders, server start, handshake), no display needed.
//!
//! Linux only (Unix-socket server, `/proc`).

#![cfg(all(feature = "gui", target_os = "linux"))]

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::process::{Command, Output};

mod common;

use common::Home;

fn check(home: &Path) -> Output {
    let run = home.join("run");
    std::fs::create_dir_all(&run).unwrap();
    // The runtime folder of a login exists (made by the system, private); the program's own folders in it and the data folder
    // do not.
    std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o700)).unwrap();
    Command::new(env!("CARGO_BIN_EXE_fs-texts"))
        .arg("--check")
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_RUNTIME_DIR", &run)
        .env("FS_LEASE_POLL_MS", "100")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .output()
        .unwrap()
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().mode() & 0o777
}

#[test]
fn a_fresh_user_starts_with_private_folders_and_the_server_comes_up() {
    let home = Home::new("fresh-user");
    let (data, run) = (
        home.path().join("data/freesynergy/wiki"),
        home.path().join("run/freesynergy"),
    );
    assert!(!data.exists() && !run.exists(), "nothing made beforehand");
    let out = check(home.path());
    let said = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{said}");
    assert_eq!(mode(&data), 0o700, "the data folder is private: {said}");
    assert_eq!(mode(&run), 0o700, "the runtime folder is private");
}

#[test]
fn a_server_that_refuses_its_folder_says_why_in_the_error() {
    let home = Home::new("refused");
    let data = home.path().join("data/freesynergy/wiki");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = check(home.path());
    assert!(!out.status.success(), "an open data folder must not work");
    let said =
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout);
    assert!(
        said.contains("alone") || said.contains("0700"),
        "the reason of the server is in the message: {said}"
    );
}
