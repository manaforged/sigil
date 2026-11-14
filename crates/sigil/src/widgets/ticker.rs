use std::fmt::Display;
use std::rc::Rc;

use unicode_segmentation::UnicodeSegmentation;

use crate::animation::{Animator, Property, Spring};
use crate::kernel::{
    Cx, IntoNode, NodeBuilder, NodeId, Role, TextBuilder, container, for_each, text,
};
use crate::layout::{JustifyContent, LayoutStyle, Overflow, Position};
use crate::reactive::{Reactive, Signal, effect, memo};
use crate::style::Paint;
use crate::widgets::{Framed, Widget, WidgetFrame};

const FACES: u8 = 10;
const FACE_PERCENT: f32 = 100.0;

/// Builder of a number whose digits roll to each new value.
#[must_use]
pub struct TickerBuilder<T: 'static> {
    value: Rc<dyn Reactive<T>>,
    format: Rc<dyn Fn(&T) -> String>,
    spring: Spring,
    frame: WidgetFrame,
}

/// A number in tabular figures whose digits roll on a spring to each new `value`, while signs, separators, and units stay still.
pub fn ticker<T: Display + 'static>(value: impl Reactive<T>) -> TickerBuilder<T> {
    TickerBuilder {
        value: Rc::new(value),
        format: Rc::new(|value: &T| value.to_string()),
        spring: Spring::default(),
        frame: WidgetFrame::new(),
    }
}

impl<T: 'static> TickerBuilder<T> {
    /// Sets how the value is written, such as `|price| format!("${price:.2}")`; the default is its `Display` form.
    pub fn format(mut self, format: impl Fn(&T) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Sets the spring that rolls each digit; the default is `Spring::SMOOTH`.
    pub const fn spring(mut self, spring: Spring) -> Self {
        self.spring = spring;
        self
    }

    fn root(self) -> NodeBuilder {
        let (value, format, spring) = (self.value, self.format, self.spring);
        let shown = memo(move || format(&value.read()));
        let places = memo(move || shown.with(|written| slots(written)));
        let line = container()
            .role(Role::Label)
            .accessible(shown)
            .text_value(shown)
            .layout(LayoutStyle::flex_row().justify_content(JustifyContent::End))
            .tabular_nums(true)
            .child(for_each(places, Clone::clone, move |slot| place(slot, shown, spring)));
        self.frame.finish(line)
    }
}

impl<T: 'static> Framed for TickerBuilder<T> {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl<T: 'static> IntoNode for TickerBuilder<T> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

impl<T: 'static> core::fmt::Debug for TickerBuilder<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TickerBuilder")
            .field("spring", &self.spring)
            .field("frame", &self.frame)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Slot {
    from_right: usize,
    mark: Option<String>,
}

fn slots(written: &str) -> Vec<Slot> {
    let mut out: Vec<Slot> = written
        .graphemes(true)
        .rev()
        .enumerate()
        .map(|(from_right, mark)| Slot {
            from_right,
            mark: digit(mark).is_none().then(|| mark.to_owned()),
        })
        .collect();
    out.reverse();
    out
}

fn digit(mark: &str) -> Option<u8> {
    let mut chars = mark.chars();
    match (chars.next(), chars.next()) {
        (Some(only), None) => only.to_digit(10).and_then(|d| u8::try_from(d).ok()),
        _ => None,
    }
}

fn digit_from_right(written: &str, from_right: usize) -> u8 {
    written
        .graphemes(true)
        .rev()
        .nth(from_right)
        .and_then(digit)
        .unwrap_or(0)
}

fn place(slot: &Slot, shown: Signal<String>, spring: Spring) -> NodeBuilder {
    let piece = match &slot.mark {
        None => roll(slot.from_right, shown, spring),
        Some(mark) => text(mark).nowrap().into(),
    };
    piece.accessibility_hidden(true)
}

fn roll(from_right: usize, shown: Signal<String>, spring: Spring) -> NodeBuilder {
    let target = memo(move || shown.with(|written| digit_from_right(written, from_right)));
    let position = Animator::spring(f32::from(target.peek()), spring);
    effect(move || position.target(f32::from(target.get())));
    let at = position.value();
    container()
        .layout(LayoutStyle::flex_column().overflow(Overflow::Hidden))
        .children((0..FACES).map(move |face| face_text(face, at)))
}

fn face_text(face: u8, at: Signal<f32>) -> TextBuilder {
    let glyph = text(face.to_string()).nowrap();
    let glyph = if face == 0 {
        glyph
    } else {
        glyph.layout(
            LayoutStyle::default()
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(0.0),
        )
    };
    glyph.drive(Property::TranslateYPercent, move || (f32::from(face) - at.get()) * FACE_PERCENT)
}
