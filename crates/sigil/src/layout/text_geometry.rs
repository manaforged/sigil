use std::cell::RefCell;
use std::rc::Rc;

use crate::hal::{HitPosition, Point, Rect};
use crate::reactive::Signal;
use crate::style::TextStyle;

pub struct TextStyleCapture {
    target: Signal<TextStyle>,
}

impl TextStyleCapture {
    #[must_use]
    pub const fn new(target: Signal<TextStyle>) -> Self {
        Self { target }
    }

    pub(crate) fn publish(&self, style: &TextStyle) {
        if self.target.peek() != *style {
            self.target.set(style.clone());
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaretGeometry {
    pub x: f32,
    pub top: f32,
    pub height: f32,
}

pub trait TextGeometry {
    fn hit(&self, text: &str, style: &TextStyle, width: Option<f32>, point: Point) -> HitPosition;

    fn caret(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        position: HitPosition,
    ) -> CaretGeometry;

    fn selection_rects(
        &self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
        selection: (HitPosition, HitPosition),
    ) -> Vec<Rect>;
}

thread_local! {
    static TEXT_GEOMETRY: RefCell<Option<Rc<dyn TextGeometry>>> = const { RefCell::new(None) };
}

pub fn set_text_geometry(geometry: Rc<dyn TextGeometry>) {
    TEXT_GEOMETRY.with(|cell| *cell.borrow_mut() = Some(geometry));
}

pub fn with_text_geometry<R>(f: impl FnOnce(&dyn TextGeometry) -> R) -> Option<R> {
    let provider = TEXT_GEOMETRY.with(|cell| cell.borrow().clone());
    provider.map(|geometry| f(geometry.as_ref()))
}
