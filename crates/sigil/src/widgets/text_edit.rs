//! Single-buffer text editing widget with newline-preserving value.
//!
//! Pick [`text_edit`] for buffers that may contain newline characters
//! (notes, scratchpads, code, multi-paragraph descriptions). Pick
//! [`text_field`](super::text_field::text_field) for strictly single-line
//! input such as form fields and search boxes. The two widgets share the
//! internal [`buffer::TextEdit`] editing primitive so caret movement,
//! selection, clipboard, and undo behave identically.
//!
//! ## Multi-line rendering
//!
//! The bound `Signal<String>` preserves newline characters verbatim;
//! every keystroke is applied to the rope correctly and the value
//! signal updates reactively. The visible line layout rebuilds
//! reactively against the rope via `NodeBuilder::children_reactive`
//! (B1): typing `Enter` after build grows the on-screen row count on
//! the next reactive-children flush (wired in `LayoutEngine::compute`
//! by B1.b).
//!
//! Vertical caret movement (`↑` / `↓`, `PageUp` / `PageDown`) is still
//! follow-up work — the multi-line render is a flex-column of
//! reactive per-line text nodes without a kernel-side line model. See
//! the dogfood gap-closure plan for the follow-up that adds a real
//! line-aware caret model.
//!
//! ## Key map
//!
//! | Key | Action |
//! |---|---|
//! | letters / digits / punctuation | insert character at caret |
//! | `Enter` | insert `\n` (newline) into the bound value signal |
//! | `Ctrl+Enter` | fire the `.submit()` callback |
//! | `Backspace` / `Delete` | delete previous / next grapheme |
//! | `←` / `→` / `Home` / `End` | move caret |
//! | `Shift` + arrow / `Home` / `End` | extend selection |
//! | `Ctrl+A` | select all |
//! | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | clipboard copy / cut / paste |
//! | `Ctrl+Z` / `Ctrl+Y` / `Ctrl+Shift+Z` | undo / redo |
//!
//! See the dogfood gap-closure plan for the vertical-caret followup.

mod buffer;
mod builder;
mod handlers;
mod render;

pub(crate) use buffer::TextEdit;
pub use builder::{TextEditBuilder, text_edit};
