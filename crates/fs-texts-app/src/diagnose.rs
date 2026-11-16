//! `fs-texts --diagnose`: what a person sends when the window stays grey: the paths, the rights, the answer of the server,
//! the version and the lease, as plain text. Needs no display and starts nothing (it never starts or ends a server).
//!
//! Pattern: Template method — `report` collects the same facts in the same order; each fact is one sentence of
//! `locales/*.mf2` (`diagnose-*`), so the head names the program or the facet in the person's language.

use std::path::Path;

use fs_interfaces::rpc::companion_path;
use fs_texts_client::RpcTexts;
use fs_texts_core::{Paging, Texts};

use crate::args::Args;
use crate::ui_texts::UiTexts;

/// The text of the diagnosis for `args` (the socket and the facet as the window would take them).
pub fn report(args: &Args) -> String {
    let german = crate::frontend::is_german(args);
    let t = UiTexts::new(german);
    let facet = args.facet.as_deref();
    let line = |what: &str, value: String| {
        t.say(
            "diagnose-line",
            &[("what", &t.plain(what)), ("value", &value)],
        )
    };
    let env = |name: &str| std::env::var(name).unwrap_or_else(|_| t.plain("diagnose-unset"));
    let mut out = vec![
        t.say(
            "diagnose-title",
            &[
                ("name", &t.program_name(facet)),
                (
                    "facet",
                    &facet.map_or_else(|| t.plain("diagnose-none"), str::to_owned),
                ),
            ],
        ),
        line("diagnose-version", env!("CARGO_PKG_VERSION").to_owned()),
        line(
            "diagnose-binary",
            std::env::current_exe().map_or_else(|e| e.to_string(), |p| p.display().to_string()),
        ),
        line("diagnose-user", user_id(&t)),
        line("diagnose-env-runtime", env("XDG_RUNTIME_DIR")),
        line("diagnose-env-data", env("XDG_DATA_HOME")),
        line("diagnose-env-home", env("HOME")),
    ];
    let socket = crate::server::socket_path(args.socket.as_deref());
    match &socket {
        Ok(socket) => {
            out.push(line("diagnose-socket", socket.display().to_string()));
            if let Some(dir) = socket.parent() {
                out.push(line("diagnose-runtime", folder(&t, dir)));
            }
            out.push(line("diagnose-socket-file", file_state(&t, socket)));
            let lease = companion_path(socket, "lease");
            out.push(line(
                "diagnose-lease",
                format!("{}: {}", lease.display(), lease_state(&t, &lease)),
            ));
            out.push(line("diagnose-server", server_answer(&t, socket)));
        }
        Err(e) => out.push(line("diagnose-socket", e.clone())),
    }
    match crate::store_path::data_dir() {
        Some(data) => {
            let wiki = data.join("freesynergy").join("wiki");
            out.push(line("diagnose-data", folder(&t, &wiki)));
        }
        None => out.push(line("diagnose-data", t.plain("diagnose-unset"))),
    }
    out.join("\n")
}

/// The owner of this process (the owner of `/proc/self`); the rights of the folders are compared with it.
fn user_id(t: &UiTexts) -> String {
    own_uid().map_or_else(|| t.plain("diagnose-unknown"), |u| u.to_string())
}

#[cfg(unix)]
fn own_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
}

#[cfg(not(unix))]
fn own_uid() -> Option<u32> {
    None
}

/// Path, owner, mode and whether a file can be made in it.
fn folder(t: &UiTexts, dir: &Path) -> String {
    let writable = if crate::start_access::writable_without_making(dir) {
        t.plain("diagnose-writable")
    } else {
        t.plain("diagnose-not-writable")
    };
    format!("{} ({}; {writable})", dir.display(), owner_mode(t, dir))
}

fn file_state(t: &UiTexts, file: &Path) -> String {
    if file.exists() {
        owner_mode(t, file)
    } else {
        t.plain("diagnose-missing")
    }
}

#[cfg(unix)]
fn owner_mode(t: &UiTexts, path: &Path) -> String {
    use std::os::unix::fs::MetadataExt;
    match std::fs::metadata(path) {
        Ok(m) => {
            let mine = own_uid() == Some(m.uid());
            let who = if mine {
                t.plain("diagnose-owner-me")
            } else {
                t.plain("diagnose-owner-other")
            };
            format!("{who} uid {}, mode {:o}", m.uid(), m.mode() & 0o7777)
        }
        Err(e) => e.to_string(),
    }
}

#[cfg(not(unix))]
fn owner_mode(_t: &UiTexts, path: &Path) -> String {
    std::fs::metadata(path).map_or_else(|e| e.to_string(), |_| String::new())
}

/// Whether the lease file can be looked at and a shared lock taken on it (what a window does at the start).
fn lease_state(t: &UiTexts, lease: &Path) -> String {
    if !lease.exists() {
        return t.plain("diagnose-missing");
    }
    match std::fs::OpenOptions::new().read(true).open(lease) {
        Ok(file) => match file.try_lock_shared() {
            Ok(()) => t.plain("diagnose-lease-ok"),
            Err(e) => t.say("diagnose-lease-held", &[("error", &e.to_string())]),
        },
        Err(e) => e.to_string(),
    }
}

/// Connect without starting anything and ask for one page: what a window would meet.
fn server_answer(t: &UiTexts, socket: &Path) -> String {
    let times = fs_texts_client::Timeouts {
        call: std::time::Duration::from_secs(5),
        ..fs_texts_client::Timeouts::default()
    };
    let texts = match RpcTexts::connect_with(socket, times) {
        Ok(texts) => texts,
        Err(e) => return t.say("diagnose-no-server", &[("error", &e.to_string())]),
    };
    let asked = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
        .and_then(|rt| {
            rt.block_on(texts.list("", Paging::new(1, 0)))
                .map_err(|e| e.to_string())
        });
    match asked {
        Ok(_) => t.plain("diagnose-answers"),
        Err(e) => t.say("diagnose-answers-not", &[("error", &e)]),
    }
}
