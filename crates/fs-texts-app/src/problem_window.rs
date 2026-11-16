//! The window that says why the wiki could not start, with a button to try again.
//!
//! Pattern: Strategy — a surface of `fs-components-app` that draws one reason (`start_problem.rs`) instead of the wiki.
//! Nothing here is grey without a sentence: the reason is in the body and in the status line, and "Try again" starts the
//! program again with the same words (a new process: a window that failed cannot become the wiki window).

use std::sync::Arc;

use fs_components::blinc::cn::components::button::{ButtonBuilder, ButtonVariant};
use fs_components::blinc::layout::div::div;
use fs_components::blinc::layout::prelude::text;
use fs_components_app::{
    AppError, AppWidget, Element, Screen, Status, Surface, WindowOptions, launch,
};

use crate::start_problem::Problem;
use crate::ui_texts::UiTexts;
use crate::window_modal::{Act, styles};

/// The sentences of the body: the title, the reason, and the hint about an older window when a server is concerned.
///
/// The title follows the program or the facet (`facet`): "Texts cannot start", or "The wiki cannot start" for `--facet wiki`.
pub fn lines(problem: &Problem, german: bool, facet: Option<&str>) -> Vec<String> {
    let t = UiTexts::new(german);
    let mut lines = vec![t.start_title(facet), problem.say(&t)];
    if matches!(problem, Problem::Server { .. }) {
        lines.push(t.plain("start-close-old"));
    }
    lines
}

struct ProblemSurface {
    lines: Vec<String>,
    retry_word: String,
    retry: Act,
}

impl Surface for ProblemSurface {
    fn build(&self, width: f32, _height: f32) -> Box<dyn Element> {
        let mut body = div().class("fs-texts-card").w(width.clamp(320.0, 760.0));
        for (i, line) in self.lines.iter().enumerate() {
            let class = match i {
                0 => "fs-nd-title",
                1 => "fs-texts-modal-error",
                _ => "fs-texts-modal-body",
            };
            body = body.child(text(line.clone()).class(class));
        }
        let retry = Arc::clone(&self.retry);
        body = body.child(
            ButtonBuilder::with_key("fs-texts-retry", self.retry_word.clone())
                .variant(ButtonVariant::Primary)
                .on_click(move |_| retry())
                .build_component()
                .id("fs-texts-retry"),
        );
        Box::new(div().class("fs-texts-problem").child(body))
    }

    fn styles(&self) -> String {
        format!(
            "{}\n.fs-texts-problem {{ padding: var(--space-6); }}\n",
            styles()
        )
    }
}

/// The window of `problem`: the surface and the status line say it; `retry` is what the button does.
///
/// # Errors
/// The shell cannot be built.
pub fn build(
    problem: &Problem,
    german: bool,
    facet: Option<&str>,
    retry: Act,
) -> Result<AppWidget, AppError> {
    let lines = lines(problem, german, facet);
    let surface = Arc::new(ProblemSurface {
        lines: lines.clone(),
        retry_word: UiTexts::new(german).plain("start-retry"),
        retry,
    });
    let app = AppWidget::new(
        "FreeSynergy Texts",
        vec![Screen::new("problem", "", surface as Arc<dyn Surface>)],
    )?;
    app.set_status(Some(Status::Problem(lines[1].clone())));
    Ok(app)
}

/// Open the window of `problem` and return when it is closed. "Try again" starts this program again with `words` and
/// closes the window.
///
/// # Errors
/// The window cannot be opened (the reason is already said on stderr).
pub fn show(
    problem: &Problem,
    german: bool,
    facet: Option<&str>,
    words: &[String],
) -> Result<(), String> {
    let words = words.to_vec();
    let slot: Arc<std::sync::OnceLock<AppWidget>> = Arc::default();
    let again = Arc::clone(&slot);
    let retry: Act = Arc::new(move || {
        if let Ok(me) = std::env::current_exe()
            && let Err(e) = std::process::Command::new(me).args(&words).spawn()
        {
            eprintln!("fs-texts: the new start failed: {e}");
            return;
        }
        if let Some(app) = again.get() {
            app.quit();
        }
    });
    let app = build(problem, german, facet, retry).map_err(|e| e.to_string())?;
    let _ = slot.set(app.clone());
    launch(WindowOptions::default(), app).map_err(|e| e.to_string())
}
