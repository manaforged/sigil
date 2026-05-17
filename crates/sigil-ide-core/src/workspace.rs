//! Workspace + fs-watch primitives (I-T6).
//!
//! - [`WorkspaceRoot`] owns one filesystem root, walks files
//!   respecting `.gitignore` + project ignore patterns, and spawns
//!   filesystem watchers.
//! - [`Watcher`] receives [`FsEvent`]s on a `crossbeam-channel`
//!   with debounce + burst coalescing.
//! - [`Project`] models the optional `.sigil-ide/workspace.toml`
//!   configuration file.
//!
//! Multi-root workspaces are not a separate struct — a consumer
//! holds a `Vec<WorkspaceRoot>` and queries each independently.
//! Overlapping roots resolve deterministically: each
//! `WorkspaceRoot` walks only files **under its own canonical
//! path**, so a file that lives under two roots appears in both
//! file iterators but never causes a double-walk inside a single
//! root.

mod project;
mod root;
mod watch;

pub use project::{LanguageConfig, Project};
pub use root::{WalkIter, WorkspaceRoot};
pub use watch::{FsEvent, Watcher};
