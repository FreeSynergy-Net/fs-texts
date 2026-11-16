//! The link of the window to its server while the window runs: when the server is gone (ended, crashed, replaced) the
//! window says so with the socket and the error, locks the entries of the menu Page WITH that reason, and offers "Reconnect".
//!
//! Pattern: Observer — a small thread of the window asks the server now and then (`Probe`); a change (reachable or not)
//! is the only thing it reports, to the status line, the menu and one modal. Nothing here is grey without a sentence
//! (rule: a missing data source or right is always reported, never silently greyed out).

use std::path::PathBuf;
use std::sync::{Arc, PoisonError};
use std::time::Duration;

use fs_components::blinc::cn::components::button::{ButtonBuilder, ButtonVariant};
use fs_components::blinc::layout::div::div;
use fs_components::blinc::layout::prelude::text;
use fs_components_app::Status;

use crate::frontend_window::Switcher;
use crate::window_modal::{Modal, View};

/// Asks the server something small: `Err` = the words of why it did not answer.
pub type Probe = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;
/// Starts the program again, as the start does (a new window that starts the server); `Err` = why it did not.
pub type Reconnect = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

/// What the window knows about its server.
pub struct Link {
    /// The question the watcher asks.
    pub probe: Probe,
    /// What "Reconnect" does; the window ends when it worked.
    pub reconnect: Reconnect,
    /// Where the server listens (shown with the error).
    pub socket: PathBuf,
    /// The facet of the window (names the program in the sentences).
    pub facet: Option<String>,
    /// Things the start found that the person must be told (a lease that could not be held).
    pub notes: Vec<String>,
    /// How often the watcher asks.
    pub every: Duration,
}

impl Switcher {
    /// Why the server cannot be used now (the sentence of the status line), none = it answers.
    pub(crate) fn lost(&self) -> Option<String> {
        self.lost
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The thread that asks the server; it ends with the window.
    pub(crate) fn watch_link(self: &Arc<Self>) {
        let Some(link) = &self.link else { return };
        if let Some(note) = link.notes.first() {
            self.problem(note.clone());
        }
        let (me, probe, every) = (Arc::downgrade(self), Arc::clone(&link.probe), link.every);
        std::thread::spawn(move || {
            loop {
                let answer = probe();
                let Some(me) = me.upgrade() else { return };
                match answer {
                    Err(error) => me.link_lost(&error),
                    Ok(()) => me.link_back(),
                }
                drop(me);
                std::thread::sleep(every);
            }
        });
    }

    /// The server did not answer: the status line, the locked menu and the modal say it (once per loss).
    pub(crate) fn link_lost(self: &Arc<Self>, error: &str) {
        let Some(link) = &self.link else { return };
        let t = self.texts();
        let path = link.socket.display().to_string();
        let name = t.program_name(link.facet.as_deref());
        let status = t.say(
            "link-lost-status",
            &[("name", &name), ("path", &path), ("error", error)],
        );
        if self.lost().as_deref() == Some(status.as_str()) {
            return;
        }
        eprintln!("fs-texts: {status}");
        *self.lost.lock().unwrap_or_else(PoisonError::into_inner) = Some(status.clone());
        self.problem(status);
        self.surface.modal.set(self.lost_modal(&path, error));
        self.redraw();
    }

    /// The server answers again (after it had not): the lock is lifted.
    pub(crate) fn link_back(self: &Arc<Self>) {
        if self
            .lost
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .is_none()
        {
            return;
        }
        self.surface.modal.clear();
        self.app
            .set_status(Some(Status::Info(self.texts().plain("link-back"))));
        self.redraw();
    }

    /// "Reconnect": the program starts again like at the start; this window ends when that worked.
    fn reconnect(self: &Arc<Self>) {
        let Some(link) = &self.link else { return };
        match (link.reconnect)() {
            Ok(()) => {
                self.app.quit();
            }
            Err(e) => self.problem(e),
        }
    }

    fn lost_modal(self: &Arc<Self>, path: &str, error: &str) -> Modal {
        let t = self.texts();
        let (title, body) = (
            t.plain("link-lost-title"),
            t.say("link-lost-body", &[("path", path), ("error", error)]),
        );
        let (again, close) = (t.plain("link-reconnect"), t.plain("link-close"));
        let (me_yes, me_no) = (Arc::clone(self), Arc::clone(self));
        let yes: Arc<dyn Fn() + Send + Sync> = Arc::new(move || me_yes.reconnect());
        let no: Arc<dyn Fn() + Send + Sync> = Arc::new(move || me_no.close_modal());
        let (on_yes, on_no) = (Arc::clone(&yes), Arc::clone(&no));
        let view: View = Arc::new(move || {
            let (y, n) = (Arc::clone(&on_yes), Arc::clone(&on_no));
            let buttons = div()
                .class("fs-nd-buttons")
                .child(
                    ButtonBuilder::with_key("fs-texts-l-close", close.clone())
                        .variant(ButtonVariant::Outline)
                        .on_click(move |_| n())
                        .build_component()
                        .id("fs-texts-l-close"),
                )
                .child(
                    ButtonBuilder::with_key("fs-texts-l-again", again.clone())
                        .variant(ButtonVariant::Primary)
                        .on_click(move |_| y())
                        .build_component()
                        .id("fs-texts-l-again"),
                );
            div().class("fs-texts-modal").child(
                div()
                    .id("fs-texts-lost")
                    .class("fs-nd")
                    .child(text(title.clone()).class("fs-nd-title"))
                    .child(text(body.clone()).class("fs-texts-modal-error"))
                    .child(buttons),
            )
        });
        Modal {
            view,
            submit: None,
            yes: Some(yes),
            no: Some(no),
            field_text: None,
        }
    }
}
