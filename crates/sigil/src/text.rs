//! Curated text-shaping surface for external host engines (feature `render`).
//!
//! Re-exports sigil's owned text engine so a host can shape a
//! [`Paragraph`](sigil_text::Paragraph) using only `sigil::text::*` —
//! never naming the internal `text` crate directly. See DR-054.

pub use sigil_hal::TextEngine;
pub use sigil_text::{Font, FontCollection, OwnedTextEngine, Paragraph, TextConfig, TextError};
