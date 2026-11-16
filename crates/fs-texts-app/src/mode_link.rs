//! The link of the plain wiki window to its server: the question it asks now and then, and what "Reconnect" does.
//!
//! Pattern: Factory — builds the `Link` of `window_link.rs` from the server connection and the arguments.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use fs_texts_client::RpcTexts;
use fs_texts_core::{Paging, Texts};

use crate::args::Args;
use crate::window_link::{Link, Probe, Reconnect};

/// How often the window asks the server.
pub const EVERY: Duration = Duration::from_secs(3);

/// What the start found that the person is told in the window (a lease that could not be held).
static NOTES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Remember `note` for the window to say.
pub fn note(note: String) {
    NOTES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(note);
}

/// The question: one small list, over the runtime of the window.
pub fn probe_of(texts: Arc<RpcTexts>, runtime: tokio::runtime::Handle) -> Probe {
    Arc::new(move || {
        runtime
            .block_on(texts.list("", Paging::new(1, 0)))
            .map(drop)
            .map_err(|e| e.to_string())
    })
}

/// "Reconnect": this program again with the same words (it starts the server like at the start); the caller ends this window.
pub fn reconnect_of() -> Reconnect {
    Arc::new(|| {
        let me = std::env::current_exe().map_err(|e| e.to_string())?;
        std::process::Command::new(me)
            .args(std::env::args_os().skip(1))
            .spawn()
            .map(drop)
            .map_err(|e| e.to_string())
    })
}

/// The link of a window over `texts` on `socket`.
pub fn link(args: &Args, socket: &Path, probe: Probe) -> Link {
    Link {
        probe,
        reconnect: reconnect_of(),
        socket: socket.to_path_buf(),
        facet: args.facet.clone(),
        notes: NOTES.lock().unwrap_or_else(PoisonError::into_inner).clone(),
        every: EVERY,
    }
}
