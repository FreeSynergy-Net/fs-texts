//! Why the window cannot start, in words the person reads in the window (rule: a missing server, folder or right is NEVER
//! a silent grey window; the window says WHAT is missing and WHAT can be done).
//!
//! Pattern: State — one enum holds the reason; the sentence of each reason is a message of `locales/*.mf2` (German and
//! English). The decision is pure; `problem_window.rs` draws it with a button to try again.

use std::path::{Path, PathBuf};

use crate::start_access::writable;
use crate::ui_texts::UiTexts;

/// Why the start stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// No socket was given and the environment names no runtime folder.
    NoSocket,
    /// The runtime folder (socket and lease) cannot be written.
    RuntimeFolder(PathBuf),
    /// The data folder of the server cannot be written.
    DataFolder(PathBuf),
    /// No server of this version could be reached or started.
    Server {
        /// The command the window ran to start the server.
        command: String,
        /// The socket it waited on.
        socket: PathBuf,
        /// What the handshake said.
        error: String,
    },
    /// Another step of the start failed (records, drafts, graphics); the words of the failure.
    Failed(String),
}

impl Problem {
    /// The id of the sentence of this reason.
    pub fn id(&self) -> &'static str {
        match self {
            Problem::NoSocket => "problem-no-socket",
            Problem::RuntimeFolder(_) => "problem-runtime-folder",
            Problem::DataFolder(_) => "problem-data-folder",
            Problem::Server { .. } => "problem-server",
            Problem::Failed(_) => "problem-failed",
        }
    }

    /// What is missing and what can be done, in the language of `t`.
    pub fn say(&self, t: &UiTexts) -> String {
        let shown = |p: &Path| p.display().to_string();
        match self {
            Problem::NoSocket => t.plain(self.id()),
            Problem::RuntimeFolder(p) | Problem::DataFolder(p) => {
                t.say(self.id(), &[("path", &shown(p))])
            }
            Problem::Server {
                command,
                socket,
                error,
            } => t.say(
                self.id(),
                &[
                    ("command", command),
                    ("path", &shown(socket)),
                    ("error", error),
                ],
            ),
            Problem::Failed(error) => t.say(self.id(), &[("error", error)]),
        }
    }
}

/// The folders the start needs before it talks to a server: the runtime folder of the socket and, when the server is
/// the window's own (no `--socket`), the data folder of the wiki. `None` = all can be written.
pub fn preflight(socket: &Path, own_socket: bool, data: Option<&Path>) -> Option<Problem> {
    if let Some(dir) = socket.parent()
        && !writable(dir)
    {
        return Some(Problem::RuntimeFolder(dir.to_path_buf()));
    }
    if !own_socket && let Some(data) = data {
        let wiki = data.join("freesynergy").join("wiki");
        if !writable(&wiki) {
            return Some(Problem::DataFolder(wiki));
        }
    }
    None
}
