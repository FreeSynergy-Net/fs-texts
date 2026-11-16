//! Pass-through of user files: a page of the user (HTML, maybe with its own script) is a
//! **original** (ADR 0030, 0086) and is copied byte for byte, never read as markup, never
//! changed, never framed.
//!
//! Pattern: Facade — `copy_file` is the one entry; behind it sit the path rule (`path`), the
//! media table and content probe (`media`), the set of all paths (`collect`), and the
//! file-system checks and the copy (`fsx`).

mod collect;
mod error;
mod fsx;
mod media;
mod path;

pub use collect::PassSet;
pub use error::PassError;
pub use fsx::{copy_file, copy_from, measure, open_plain, Copied, Limits};
pub use media::{MediaType, Prober};
pub use path::PassPath;
