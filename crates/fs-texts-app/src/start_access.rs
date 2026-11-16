//! Whether the window may change pages at the start, and when not: what is missing and what can be done.
//!
//! Pattern: none — pure decisions over the arguments and the data folder. The rule of FreeSynergy: when a data folder,
//! a source or a permission is missing, the program says WHAT is missing and WHAT can be done; it is never silently
//! greyed out or silently read only.

use std::path::Path;

use crate::args::Args;
use crate::ui_texts::UiTexts;

/// Why the window starts read only, with the way out; none = pages can be made and edited.
///
/// `data` is the folder the window keeps its own files in (records, drafts): when nothing can be written there, nothing can
/// be saved either.
pub fn start_lock(args: &Args, data: &Path, german: bool) -> Option<String> {
    let t = UiTexts::new(german);
    if args.read_only {
        return Some(t.plain("reason-read-only"));
    }
    if !args.sources.is_empty() && !args.edit {
        return Some(t.plain("reason-read-only-sources"));
    }
    if !writable(data) {
        return Some(t.say("reason-no-write", &[("path", &data.display().to_string())]));
    }
    None
}

/// Whether a file can be made in `folder` (the folder is made when it is not there).
pub fn writable(folder: &Path) -> bool {
    if std::fs::create_dir_all(folder).is_err() {
        return false;
    }
    // A fresh name each time (`create_new`): a leftover file of an earlier run can never stand for write access.
    for n in 0..16_u32 {
        let probe = folder.join(format!(".fs-texts-write-test-{}-{n}", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
        {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                return true;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return false,
        }
    }
    false
}
