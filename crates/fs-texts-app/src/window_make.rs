//! Making the wiki window: the shell, the surface, the switcher with its menus, and the watch of the server.
//!
//! Pattern: Factory — one function per way to make the window (plain, with the slot of its modal, with a link to the server).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use fs_components_app::{AppError, AppWidget, Screen, Status, Surface};
use fs_components_workbenches::ReaderWidget;

use crate::frontend::Frontend;
use crate::frontend_window::{Setup, Switcher};
use crate::window_link::Link;
use crate::window_modal::ModalSlot;
use crate::window_surface::ReaderSurface;

/// The window, not yet opened: the reader in the frontend `shown`, with the menus.
///
/// # Errors
/// The shell cannot be built.
pub fn window(
    title: &str,
    reader: ReaderWidget,
    shown: Frontend,
    setup: Setup,
) -> Result<AppWidget, AppError> {
    window_with_modal(title, reader, shown, setup).map(|(app, _)| app)
}

/// [`window`], and the slot of its modal: what a test (or another part of the program) uses to answer a dialog without a
/// click (`ModalSlot::submit`, `yes`, `no`).
///
/// # Errors
/// The shell cannot be built.
pub fn window_with_modal(
    title: &str,
    reader: ReaderWidget,
    shown: Frontend,
    setup: Setup,
) -> Result<(AppWidget, Arc<ModalSlot>), AppError> {
    window_linked(title, reader, shown, setup, None)
}

/// [`window_with_modal`] with a link to the server (`window_link.rs`): the window watches it and says when it is lost.
///
/// # Errors
/// The shell cannot be built.
pub fn window_linked(
    title: &str,
    reader: ReaderWidget,
    shown: Frontend,
    setup: Setup,
    link: Option<Link>,
) -> Result<(AppWidget, Arc<ModalSlot>), AppError> {
    setup.bare.store(shown.has_own_head(), Ordering::Relaxed);
    let widget = reader.clone().layout(shown.layout(), setup.labels.clone());
    let hint = reader.with_reader(|r| r.labels().search.clone());
    let surface = Arc::new(ReaderSurface::new(
        widget,
        &setup.labels.program,
        &hint,
        shown,
    ));
    let modal = Arc::clone(&surface.modal);
    let screen = Screen::new("reader", title, Arc::clone(&surface) as Arc<dyn Surface>);
    let app = AppWidget::new(title, vec![screen])?;
    let switcher = Arc::new(Switcher {
        app: app.clone(),
        surface,
        base: reader,
        setup,
        shown: Mutex::new(shown),
        fresh: Mutex::default(),
        menu_state: Mutex::default(),
        listed: Mutex::default(),
        link,
        lost: Mutex::default(),
        reconnecting: AtomicBool::new(false),
    });
    *switcher
        .listed
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = switcher.list_now();
    switcher.watch_saves();
    switcher.set_empty_state();
    switcher.install(shown)?;
    // Locked for a reason: the window says it in the status line too, not only in the grey entries.
    if let Some(why) = switcher
        .setup
        .pages
        .as_ref()
        .and_then(|p| p.locked_reason())
    {
        app.set_status(Some(Status::Problem(why)));
    }
    switcher.watch_state();
    switcher.watch_link();
    Ok((app, modal))
}
