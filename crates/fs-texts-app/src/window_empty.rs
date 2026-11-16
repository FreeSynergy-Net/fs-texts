//! What the window shows while the wiki has no page at all: a sentence and a button, in the middle of the page and in the
//! place of the page tree. Nothing is left empty and grey (rule: a missing thing says why and what to do).
//!
//! Pattern: State — the surface draws this view instead of the reader's layout while the list of pages is empty and no
//! page is open; the moment a page exists the reader's own layout is drawn again. The view is at rest in the area below
//! the bar of the window, so an open menu (it drops from the bar, over the top of the page) does not cover the sentence.

use std::sync::{Arc, PoisonError};

use fs_components::blinc::cn::components::button::{ButtonBuilder, ButtonVariant};
use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::layout::prelude::text;

use crate::frontend_window::{PAGE_NEW, Switcher};

/// The words of the empty state.
#[derive(Clone)]
pub(crate) struct EmptyWords {
    /// "Pages" (the heading of the tree).
    pub(crate) pages: String,
    /// "No pages yet."
    pub(crate) title: String,
    /// "Make the first page." (or why that cannot be done here).
    pub(crate) hint: String,
    /// "New page" (the word of the button).
    pub(crate) new_word: String,
    /// The button is drawn only when a page can be made here.
    pub(crate) can_make: bool,
}

/// What a click on the button does.
pub(crate) type OnNew = Arc<dyn Fn() + Send + Sync>;

fn make_button(tag: &str, words: &EmptyWords, on_new: &OnNew) -> Option<Div> {
    if !words.can_make {
        return None;
    }
    let on_new = Arc::clone(on_new);
    Some(
        div().child(
            ButtonBuilder::with_key(format!("fs-texts-empty-new-{tag}"), words.new_word.clone())
                .variant(ButtonVariant::Primary)
                .on_click(move |_| on_new())
                .build_component()
                .id(&format!("fs-texts-empty-new-{tag}")),
        ),
    )
}

/// The view: the page tree's place on the left says "No pages yet" with the button, the page's place says it again, centred.
pub(crate) fn empty_view(words: &EmptyWords, on_new: &OnNew, width: f32, height: f32) -> Div {
    let side_width = (width * 0.25).clamp(180.0, 300.0).min(width);
    let mut side = div()
        .id("fs-texts-empty-side")
        .class("fs-wk-b-side")
        .w(side_width)
        .h_full()
        .flex_col()
        .flex_shrink_0()
        .gap(8.0)
        .p(16.0)
        .child(text(words.pages.clone()).class("fs-wk-b-section-text"))
        .child(text(words.title.clone()));
    if let Some(button) = make_button("side", words, on_new) {
        side = side.child(button);
    }
    let mut middle = div()
        .id("fs-texts-empty-page")
        .flex_1()
        .h_full()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(12.0)
        .child(
            text(words.title.clone())
                .size(22.0)
                .id("fs-texts-empty-title"),
        )
        .child(text(words.hint.clone()).id("fs-texts-empty-hint"));
    if let Some(button) = make_button("page", words, on_new) {
        middle = middle.child(button);
    }
    div()
        .class("fs-wk-root")
        .w(width)
        .h(height)
        .flex_row()
        .child(side)
        .child(middle)
}

impl Switcher {
    /// Why pages cannot be changed now: the server is gone, or the wiki is read only (the reason of the locked menu entries).
    pub(crate) fn why_locked(&self) -> Option<String> {
        let t = self.texts();
        self.lost()
            .map(|_| t.plain("reason-lost"))
            .or_else(|| self.setup.pages.as_ref().and_then(|p| p.locked_reason()))
    }

    /// The words and the button of the view that stands in for an empty wiki (`window_empty.rs`).
    pub(crate) fn set_empty_state(self: &Arc<Self>) {
        let (words, click) = (Arc::downgrade(self), Arc::downgrade(self));
        let words = move || {
            let me = words.upgrade();
            let german = me.as_ref().is_some_and(|m| m.setup.german);
            let t = crate::ui_texts::UiTexts::new(german);
            let why = me.as_ref().and_then(|m| {
                let locked = m.setup.pages.as_ref().and_then(|p| p.locked_reason());
                m.lost().map(|_| t.plain("reason-lost")).or(locked)
            });
            EmptyWords {
                pages: me
                    .as_ref()
                    .map_or_else(String::new, |m| m.setup.labels.pages.clone()),
                title: t.plain("empty-title"),
                hint: why.clone().unwrap_or_else(|| t.plain("empty-hint")),
                new_word: t.plain("page-new"),
                can_make: why.is_none()
                    && me
                        .as_ref()
                        .is_some_and(|m| m.setup.pages.as_ref().is_some_and(|p| p.can_create())),
            }
        };
        let on_new: OnNew = Arc::new(move || {
            if let Some(me) = click.upgrade() {
                me.page_action(PAGE_NEW);
            }
        });
        let why = Arc::downgrade(self);
        *self
            .surface
            .banner
            .lock()
            .unwrap_or_else(PoisonError::into_inner) =
            Some(Arc::new(move || why.upgrade()?.why_locked()));
        *self
            .surface
            .empty
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some((Arc::new(words), on_new));
    }
}
