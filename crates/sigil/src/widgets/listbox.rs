use std::rc::Rc;

use crate::events::prelude::{Key, NodeEventsExt};
use crate::hal::{KeyEvent, LogicalKey};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, signal};
use crate::widgets::scroll::{ScrollBuilder, reveal_row, scroll};

pub(crate) trait ListModel: 'static {
    fn cursor(&self) -> Signal<usize>;
    fn len(&self) -> usize;
    fn enabled(&self, _index: usize) -> bool {
        true
    }
    fn label(&self, _index: usize) -> Option<String> {
        None
    }
    fn is_open(&self) -> bool {
        true
    }
    fn open(&self) {}
    fn commit(&self, index: usize);
    fn close(&self);
    fn blur(&self) {
        self.close();
    }
    fn text_input(&self) -> bool {
        false
    }
}

#[must_use]
pub(crate) fn listbox(node: NodeBuilder, model: impl ListModel) -> NodeBuilder {
    let model: Rc<dyn ListModel> = Rc::new(model);
    let down = Rc::clone(&model);
    let up = Rc::clone(&model);
    let enter = Rc::clone(&model);
    let escape = Rc::clone(&model);
    let blur = Rc::clone(&model);
    let other = model;
    node.key(Key::ArrowDown, move |event| {
        event.stop_propagation();
        if down.is_open() {
            step(down.as_ref(), true);
        } else {
            down.open();
        }
    })
    .key(Key::ArrowUp, move |event| {
        event.stop_propagation();
        if up.is_open() {
            step(up.as_ref(), false);
        }
    })
    .key(Key::Enter, move |event| {
        event.stop_propagation();
        commit_or_open(enter.as_ref());
    })
    .key(Key::Escape, move |event| {
        event.stop_propagation();
        escape.close();
    })
    .key_any(move |event| other_key(other.as_ref(), event))
    .focus_out(move || blur.blur())
}

fn commit_or_open(model: &dyn ListModel) {
    if !model.is_open() {
        model.open();
        return;
    }
    let index = model.cursor().peek();
    if index < model.len() && model.enabled(index) {
        model.commit(index);
    }
}

fn other_key(model: &dyn ListModel, event: &mut KeyEvent) {
    if model.text_input() || !model.is_open() {
        return;
    }
    let moved = match event.key {
        LogicalKey::Named(Key::Home) => jump(model, false),
        LogicalKey::Named(Key::End) => jump(model, true),
        _ => typed_char(event).is_some_and(|ch| type_ahead(model, ch)),
    };
    if moved {
        event.stop_propagation();
    }
}

fn typed_char(event: &KeyEvent) -> Option<char> {
    if event.modifiers.ctrl() || event.modifiers.meta() || event.modifiers.alt() {
        return None;
    }
    let mut chars = event.text.as_deref()?.chars();
    let ch = chars.next()?;
    (chars.next().is_none() && !ch.is_control() && !ch.is_whitespace()).then_some(ch)
}

pub(crate) fn step(model: &dyn ListModel, forward: bool) {
    let len = model.len();
    let from = model.cursor().peek();
    let next = if forward {
        let start = if from >= len { 0 } else { from + 1 };
        (start..len).find(|&i| model.enabled(i))
    } else {
        let end = from.min(len);
        (0..end).rev().find(|&i| model.enabled(i))
    };
    if let Some(index) = next {
        model.cursor().set(index);
    }
}

fn jump(model: &dyn ListModel, last: bool) -> bool {
    let len = model.len();
    let found = if last {
        (0..len).rev().find(|&i| model.enabled(i))
    } else {
        (0..len).find(|&i| model.enabled(i))
    };
    found.is_some_and(|index| {
        model.cursor().set(index);
        true
    })
}

fn type_ahead(model: &dyn ListModel, ch: char) -> bool {
    let len = model.len();
    if len == 0 {
        return false;
    }
    let needle = ch.to_lowercase().collect::<String>();
    let from = model.cursor().peek();
    let found = (1..=len)
        .map(|offset| (from.min(len - 1) + offset) % len)
        .find(|&i| {
            model.enabled(i)
                && model
                    .label(i)
                    .is_some_and(|label| label.to_lowercase().starts_with(&needle))
        });
    found.is_some_and(|index| {
        model.cursor().set(index);
        true
    })
}

#[must_use]
pub(crate) fn scrolled(
    list: NodeBuilder,
    cursor: Signal<usize>,
    row_height: f32,
    extent: f32,
    viewport: (Option<f32>, f32),
) -> ScrollBuilder {
    let scroll_y = signal(0.0_f32);
    reveal_row(scroll_y, cursor, row_height, viewport.1);
    let view = scroll(list, extent)
        .height(viewport.1)
        .scroll_bind(scroll_y);
    match viewport.0 {
        Some(width) => view.width(width),
        None => view,
    }
}
