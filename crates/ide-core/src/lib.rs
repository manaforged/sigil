//! `sigil-ide-core` — IDE-grade primitives for sigil consumers.
//!
//! This crate is the substrate for the host engine's IDE-grade editor and any
//! future IDE consumer of sigil. Each module is a separable primitive:
//!
//! - [`buffer`] — rope-backed text buffer + history + multi-cursor (I-T2/I-T3/I-T4)
//! - [`workspace`] — workspace model + fs-watch (I-T6)
//! - [`search`] — fuzzy-search lib (I-T5)
//! - [`settings`] — settings store with TOML hot-reload (I-T7)
//! - [`keymap`] — keymap parser + binding resolver (I-T8)
//!
//! **One-way dep rule:** this crate MUST NOT depend on `sigil` (the
//! widget library). The CI guard at `crates/sigil-ide-core/build.rs`
//! enforces it. Primitives stay library-quality so they're consumable
//! beyond sigil's widget surface.

// Dev-dep crates used only in benches/tests — silence the unused-crate lint.
#[cfg(test)]
use criterion as _;
#[cfg(test)]
use proptest as _;
#[cfg(test)]
use ropey as _;
#[cfg(test)]
use tempfile as _;

pub mod buffer;
pub mod keymap;
pub mod search;
pub mod settings;
pub mod workspace;
