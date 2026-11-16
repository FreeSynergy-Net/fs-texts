//! The window of the reading wiki: the menu View > Frontend switches between Classic, Confluence and Notion while it runs.
//!
//! Pattern: Strategy in a shell — the reader of `fs-components` is the one surface; a choice in the menu puts the same
//! reader (its state is shared by the clones) into another layout, remembers the choice (`frontend.rs`) and redraws. The
//! menu Page (`window_menu.rs`) and what it does (`window_pages.rs`, dialogs in `window_modal.rs`) belong to the same window.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use fs_components_app::{AppError, AppWidget, Status};
use fs_components_workbenches::{ReaderWidget, WikiLabels};

use crate::frontend::{self, Frontend};
use crate::page_words::page_words;
use crate::source::PageMaker;
use crate::ui_texts::UiTexts;
use crate::window_link::Link;
pub use crate::window_surface::EDITOR_STYLES;
use crate::window_surface::ReaderSurface;

pub use crate::window_make::{window, window_linked, window_with_modal};
pub use crate::window_menu::{
    FRONTEND_PREFIX, PAGE_DELETE, PAGE_DISCARD, PAGE_NEW, PAGE_RENAME, PAGE_SUB, PageState, QUIT,
    menus, page_menu,
};

/// What the window needs besides the reader.
pub struct Setup {
    /// The words of the layouts (the frontend changes the layout, not the words).
    pub labels: WikiLabels,
    /// Whether the menus speak German.
    pub german: bool,
    /// The switch of the source: a frontend with its own head draws the language bar itself.
    pub bare: Arc<AtomicBool>,
    /// Where the choice is remembered; none = not remembered (a run with `--frontend`).
    pub settings: Option<PathBuf>,
    /// Makes the pages of the menu Page; none = the window has no such menu (a source that cannot make pages).
    pub pages: Option<Arc<dyn PageMaker>>,
}

/// What a choice in the menu does.
pub(crate) struct Switcher {
    pub(crate) app: AppWidget,
    pub(crate) surface: Arc<ReaderSurface>,
    pub(crate) base: ReaderWidget,
    pub(crate) setup: Setup,
    pub(crate) shown: Mutex<Frontend>,
    /// Pages made in this window and not saved since: path -> the revision they had when made.
    pub(crate) fresh: Mutex<HashMap<String, String>>,
    /// The state the menus were last set for.
    pub(crate) menu_state: Mutex<Option<PageState>>,
    /// The list (path, title) the page tree was last made from.
    pub(crate) listed: Mutex<Vec<(String, String)>>,
    /// What the window knows about its server; none = no watch (a picture, a test).
    pub(crate) link: Option<Link>,
    /// Why the server cannot be used now (the sentence of the status line); none = it answers.
    pub(crate) lost: Mutex<Option<String>>,
    /// "Reconnect" was chosen and is under way (a second click does nothing: it would open a second window).
    pub(crate) reconnecting: AtomicBool,
}

impl Switcher {
    pub(crate) fn texts(&self) -> UiTexts {
        UiTexts::new(self.setup.german)
    }

    /// What decides the entries of the menu Page now.
    fn page_state(&self) -> PageState {
        let (open, editing) = self
            .surface
            .current()
            .with_reader(|r| (r.current_path().is_some(), r.editing().is_some()));
        PageState {
            can_edit: self.setup.pages.as_ref().is_some_and(|p| p.can_create())
                && self.lost().is_none(),
            open,
            editing,
        }
    }

    pub(crate) fn install(self: &Arc<Self>, shown: Frontend) -> Result<(), AppError> {
        let me = Arc::clone(self);
        let mut all = menus(shown, self.setup.german);
        let state = self.page_state();
        if self.setup.pages.is_some() {
            let mut words = page_words(self.setup.german);
            if let Some(why) = self.setup.pages.as_ref().and_then(|p| p.locked_reason()) {
                words.read_only = why;
            }
            // The server is gone: that is the reason of the locked entries, before any other.
            if self.lost().is_some() {
                words.read_only = self.texts().plain("reason-lost");
            }
            all.insert(1, page_menu(&words, state));
        }
        *self
            .menu_state
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(state);
        self.app.set_menus(all, move |id| me.chosen(id))
    }

    /// The list of the reader as the tree shows it.
    pub(crate) fn list_now(&self) -> Vec<(String, String)> {
        self.base.with_reader(|r| {
            r.entries()
                .iter()
                .map(|e| (e.path.clone(), e.title.clone()))
                .collect()
        })
    }

    /// Make the page tree again from the list of the reader (the list read now is what the tree shows) and draw.
    fn remake_tree(&self) {
        let layout = self.shown().layout();
        drop(self.base.clone().layout(layout, self.setup.labels.clone()));
        *self.listed.lock().unwrap_or_else(PoisonError::into_inner) = self.list_now();
        self.base.refresh();
    }

    /// Set the menus again when what decides them changed (a page was opened, an edit began or ended): the reader changes by
    /// clicks the window does not see, so a small thread of the window looks now and then. A menu that is open is left
    /// alone (setting menus closes it); the next look sets it.
    pub(crate) fn watch_state(self: &Arc<Self>) {
        let me = Arc::downgrade(self);
        std::thread::spawn(move || {
            while let Some(me) = me.upgrade() {
                let open = me
                    .app
                    .with_shell(fs_components_app::Shell::open_menu)
                    .is_some();
                // The list was read again by whoever changed the pages (the reader holds it, the tree is a copy): the tree follows.
                let tree_old =
                    *me.listed.lock().unwrap_or_else(PoisonError::into_inner) != me.list_now();
                if tree_old {
                    // A tree that cannot be drawn now (no graphics set up yet) must not end this look: the menus below
                    // follow the reader all the same (the tree itself is made, only the draw failed).
                    let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        me.remake_tree();
                    }));
                    if made.is_err() {
                        let said = me.texts().plain("tree-not-drawn");
                        eprintln!("fs-texts: {said}");
                        me.problem(said);
                    }
                }
                let last = *me.menu_state.lock().unwrap_or_else(PoisonError::into_inner);
                if !open
                    && last != Some(me.page_state())
                    && me.setup.pages.is_some()
                    && let Err(e) = me.install(me.shown())
                {
                    me.menus_failed(&e);
                }
                drop(me);
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        });
    }

    fn chosen(self: &Arc<Self>, id: &str) {
        match id {
            QUIT => {
                self.app.quit();
            }
            PAGE_NEW | PAGE_SUB | PAGE_RENAME | PAGE_DELETE | PAGE_DISCARD => self.page_action(id),
            _ => {
                if let Some(chosen) = id.strip_prefix(FRONTEND_PREFIX).and_then(Frontend::parse) {
                    self.show(chosen);
                }
            }
        }
    }

    /// The reader in the layout of `chosen`: the page tree is made again from the list of the source (a new page is in it).
    pub(crate) fn relayout(self: &Arc<Self>, chosen: Frontend) {
        self.setup
            .bare
            .store(chosen.has_own_head(), Ordering::Relaxed);
        self.surface
            .confluence
            .store(chosen == Frontend::Confluence, Ordering::Relaxed);
        let widget = self
            .base
            .clone()
            .layout(chosen.layout(), self.setup.labels.clone());
        *self.listed.lock().unwrap_or_else(PoisonError::into_inner) = self.list_now();
        // The page is read again: the plain frontend has the language bar in the text of the page, the others draw it
        // themselves. An edit in progress is left alone.
        widget.with_reader(|r| {
            if r.editing().is_none()
                && let Some(path) = r.current_path().map(str::to_owned)
            {
                r.open(&path);
            }
        });
        *self
            .surface
            .widget
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = widget;
        *self.shown.lock().unwrap_or_else(PoisonError::into_inner) = chosen;
    }

    /// The page tree is made again after every saved edit: the title of the page may be another one now (it is the first
    /// heading). The tree is shared by the clones of the reader widget, so making it on a clone is enough; the save still holds
    /// the reader, so this waits in a thread of its own for the end of the save, then draws the page and the tree again.
    pub(crate) fn watch_saves(self: &Arc<Self>) {
        let Some(pages) = &self.setup.pages else {
            return;
        };
        let me = Arc::downgrade(self);
        pages.on_saved(Arc::new(move || {
            if let Some(me) = me.upgrade() {
                std::thread::spawn(move || {
                    me.remake_tree();
                    // The status line still says "Page made ... then save" otherwise (finding of the review of 0.0.13).
                    let said = me.texts().plain("msg-saved");
                    me.app.set_status(Some(Status::Info(said)));
                });
            }
        }));
    }

    pub(crate) fn shown(&self) -> Frontend {
        *self.shown.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn show(self: &Arc<Self>, chosen: Frontend) {
        self.relayout(chosen);
        if let Some(file) = &self.setup.settings
            && let Err(e) = frontend::save(file, chosen)
        {
            eprintln!(
                "fs-texts: the frontend is not remembered ({}: {e})",
                file.display()
            );
            let said = self.texts().plain("msg-frontend-not-kept");
            self.app.set_status(Some(Status::Problem(said)));
        }
        // Setting the menus draws everything again, with the mark on the new frontend.
        if let Err(e) = self.install(chosen) {
            self.menus_failed(&e);
        }
    }
}
