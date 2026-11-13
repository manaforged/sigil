use std::cell::RefCell;
use std::rc::Rc;

use crate::layout::TextMeasure;
use crate::reactive::Signal;
use crate::style::TextStyle;

pub(crate) struct TextStyleCapture {
    target: Signal<TextStyle>,
}

impl TextStyleCapture {
    #[must_use]
    pub(crate) const fn new(target: Signal<TextStyle>) -> Self {
        Self { target }
    }

    pub(crate) fn publish(&self, style: &TextStyle) {
        if self
            .target
            .try_peek()
            .is_some_and(|current| current != *style)
        {
            self.target.try_set(style.clone());
        }
    }
}

/// Holds the position and height of a text caret.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaretGeometry {
    /// The horizontal position of the caret within the text, in pixels.
    pub x: f32,
    /// The top of the caret within the text, in pixels.
    pub top: f32,
    /// The height of the caret, in pixels.
    pub height: f32,
}

thread_local! {
    static TEXT_GEOMETRY: RefCell<Option<Rc<dyn TextMeasure>>> = const { RefCell::new(None) };
}

#[cfg(feature = "kittest")]
pub fn text_geometry(geometry: Rc<dyn TextMeasure>) {
    TEXT_GEOMETRY.with(|cell| *cell.borrow_mut() = Some(geometry));
}

pub(crate) fn swap_geometry(geometry: Option<Rc<dyn TextMeasure>>) -> Option<Rc<dyn TextMeasure>> {
    TEXT_GEOMETRY.replace(geometry)
}

pub(crate) fn with_text_geometry<R>(f: impl FnOnce(&dyn TextMeasure) -> R) -> Option<R> {
    let provider = TEXT_GEOMETRY.with(|cell| cell.borrow().clone());
    provider.map(|geometry| f(geometry.as_ref()))
}
