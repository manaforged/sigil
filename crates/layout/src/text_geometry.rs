//! Pointer→glyph geometry hook for text widgets.
//!
//! Mirrors [`TextMeasure`](crate::TextMeasure): the owned text engine lives in
//! the embedding host (`app`), so core widgets reach caret/selection geometry
//! through this injected trait instead of depending on the `text` crate
//! directly. That keeps widgets renderer-agnostic per DR-054 — the same reason
//! `sigil::text` is gated behind the `render` feature.
//!
//! The host installs a provider once via [`set_text_geometry`]; widgets read it
//! during pointer handling and overlay layout via [`with_text_geometry`]. When
//! no provider is installed (headless layout tests) reads return `None`, so a
//! widget degrades to "keyboard selection only" rather than panicking.

use std::cell::RefCell;
use std::rc::Rc;

use style::TextStyle;

/// Geometry of a caret at a byte offset within a single laid-out line.
///
/// All values are in logical pixels, relative to the text's top-left content
/// origin (the same origin a `pointer_down` on the text node reports).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaretGeometry {
    /// X offset of the caret from the text's left edge.
    pub x: f32,
    /// Y offset of the caret box top from the text's top edge.
    pub top: f32,
    /// Height of the caret box (the line height).
    pub height: f32,
}

/// Pointer→glyph geometry for single-line text widgets.
///
/// Implemented by the host over the same owned text engine that backs
/// [`TextMeasure`](crate::TextMeasure), so geometry is identical to the painted
/// glyphs (text shaping is deterministic for a given text + style + fonts).
pub trait TextGeometry {
    /// Byte index of the cluster boundary nearest local `x` within `text`
    /// rendered with `style`. Always a UTF-8 boundary in `[0, text.len()]`.
    fn hit(&self, text: &str, style: &TextStyle, x: f32) -> usize;

    /// Caret geometry at `byte` within `text` rendered with `style`.
    fn caret(&self, text: &str, style: &TextStyle, byte: usize) -> CaretGeometry;
}

thread_local! {
    static TEXT_GEOMETRY: RefCell<Option<Rc<dyn TextGeometry>>> = const { RefCell::new(None) };
}

/// Install the per-thread text geometry provider.
///
/// Called once by the host during launch. UI runs single-threaded, so a
/// thread-local handoff matches the framework's `!Send` model (the same shape
/// as the clipboard and modal-stack thread-locals).
pub fn set_text_geometry(geometry: Rc<dyn TextGeometry>) {
    TEXT_GEOMETRY.with(|cell| *cell.borrow_mut() = Some(geometry));
}

/// Run `f` with the installed text geometry, returning `None` if none is set.
///
/// The provider `Rc` is cloned out before `f` runs so the thread-local borrow
/// is never held across user code.
pub fn with_text_geometry<R>(f: impl FnOnce(&dyn TextGeometry) -> R) -> Option<R> {
    let provider = TEXT_GEOMETRY.with(|cell| cell.borrow().clone());
    provider.map(|geometry| f(geometry.as_ref()))
}
