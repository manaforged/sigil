use std::rc::Rc;

use crate::events::{Key, KeyEvent, LogicalKey};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, signal};
use crate::widgets::Widget;

use crate::widgets::scroll::{reveal_row, scroll};

mod filter;
mod nav;
mod selection;
mod state;

pub use state::{CursorEdge, ListState};

pub(crate) use filter::Matcher;
pub(crate) use nav::{LIST, Move, RADIO_GROUP, RowSource, TAB_LIST, TREE};
pub(crate) use selection::{SelectMark, follow_cursor, mark, only, span, toggle, toggle_all};

use nav::LISTBOX;

/// The rows behind a list that keys move through.
pub(crate) trait ListModel: 'static {
    /// Returns the cursor.
    fn cursor(&self) -> Signal<usize>;
    /// Returns the len.
    fn len(&self) -> usize;
    /// Returns whether the item at `index` can be chosen.
    fn enabled(&self, _index: usize) -> bool {
        true
    }
    /// Returns the label of the item at `index`.
    fn label(&self, _index: usize) -> Option<String> {
        None
    }
    /// Returns whether this list model is open.
    fn is_open(&self) -> bool {
        true
    }
    /// Runs when the list opens.
    fn open(&self) {}
    /// Runs when the user chooses the item at `index`.
    fn commit(&self, index: usize);
    /// Runs when the list closes.
    fn close(&self);
    /// Runs when the list loses focus.
    fn blur(&self) {
        self.close();
    }
    /// Returns whether text input is set.
    fn text_input(&self) -> bool {
        false
    }
}

/// Route list keys on `node` to `model`.
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
            apply(down.as_ref(), Move::Next);
        } else {
            down.open();
        }
    })
    .key(Key::ArrowUp, move |event| {
        event.stop_propagation();
        if up.is_open() {
            apply(up.as_ref(), Move::Prev);
        }
    })
    .key(Key::Enter, move |event| {
        event.stop_propagation();
        commit_or_open(enter.as_ref());
    })
    .key(Key::Escape, move |event| {
        if escape.is_open() {
            event.stop_propagation();
            escape.close();
        }
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
        LogicalKey::Named(key @ (Key::Home | Key::End)) => {
            LISTBOX.step_for(key).is_some_and(|step| apply(model, step))
        }
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

struct ModelRows<'a>(&'a dyn ListModel);

impl RowSource for ModelRows<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn enabled(&self, index: usize) -> bool {
        self.0.enabled(index)
    }
}

fn apply(model: &dyn ListModel, step: Move) -> bool {
    let found = LISTBOX.target(&ModelRows(model), model.cursor().peek(), step);
    found.inspect(|&index| model.cursor().set(index)).is_some()
}

fn type_ahead(model: &dyn ListModel, ch: char) -> bool {
    let len = model.len();
    if len == 0 {
        return false;
    }
    let needle = ch.to_string();
    let from = model.cursor().peek();
    let found = (1..=len)
        .map(|offset| (from.min(len - 1) + offset) % len)
        .find(|&i| {
            model.enabled(i)
                && model
                    .label(i)
                    .is_some_and(|label| Matcher::Prefix.score(&needle, &label).is_some())
        });
    found.inspect(|&index| model.cursor().set(index)).is_some()
}

#[must_use]
pub(crate) fn scrolled(
    list: NodeBuilder,
    cursor: Signal<usize>,
    row_height: f32,
    extent: f32,
    viewport: (Option<f32>, f32),
) -> NodeBuilder {
    let scroll_y = signal(0.0_f32);
    reveal_row(scroll_y, cursor, row_height, viewport.1);
    let view = scroll(list)
        .extent(extent)
        .height(viewport.1)
        .scroll_bind(scroll_y);
    let view = match viewport.0 {
        Some(width) => view.width(width),
        None => view,
    };
    view.into_node_builder().focusable(false)
}
