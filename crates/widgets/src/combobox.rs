use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{NodeBuilder, Role, TopLayer, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Overflow, Position};
use sigil_reactive::{Signal, signal};
use sigil_style::{NodeStyleExt, StyleAtom};

use super::layout_ext::WidgetLayoutExt;
use super::scroll::scroll;
use super::text_field::{TextFieldBuilder, text_field};

const MAX_VISIBLE: usize = 8;
const ROW_HEIGHT: f32 = 28.0;
const FIELD_HEIGHT: f32 = 32.0;

static SURFACE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static SURFACE_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static OPTION_ACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

#[derive(Clone, Copy)]
pub struct Combobox {
    options: Signal<Vec<String>>,
    query: Signal<String>,
    selected: Signal<Option<usize>>,
    open: Signal<bool>,
    cursor: Signal<usize>,
    session: Signal<u32>,
    placeholder: &'static str,
}

impl Combobox {
    #[must_use]
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options: signal(options),
            query: signal(String::new()),
            selected: signal(None),
            open: signal(false),
            cursor: signal(0),
            session: signal(0),
            placeholder: "",
        }
    }

    #[must_use]
    pub const fn placeholder(mut self, text: &'static str) -> Self {
        self.placeholder = text;
        self
    }

    pub fn query(self) -> String {
        self.query.get()
    }

    pub fn selected(self) -> Option<usize> {
        self.selected.get()
    }

    pub fn selected_option(self) -> Option<String> {
        self.selected
            .get()
            .and_then(|index| self.options.peek().get(index).cloned())
    }

    pub const fn selected_signal(self) -> Signal<Option<usize>> {
        self.selected
    }

    pub fn is_open(self) -> bool {
        self.open.get()
    }

    pub fn open(self) {
        let row = self
            .selected
            .peek()
            .and_then(|index| self.filtered().iter().position(|(i, _)| *i == index))
            .unwrap_or(0);
        self.cursor.set(row);
        self.open.set(true);
    }

    pub fn close(self) {
        self.open.set(false);
    }

    pub fn filtered(self) -> Vec<(usize, String)> {
        let query = self.query.get().to_lowercase();
        self.options
            .get()
            .iter()
            .enumerate()
            .filter(|(_, label)| query.is_empty() || label.to_lowercase().contains(&query))
            .map(|(index, label)| (index, label.clone()))
            .collect()
    }

    pub fn cursor(self) -> usize {
        self.cursor.get()
    }

    pub fn edit_query(self, text: &str) {
        self.query.update(|q| {
            q.clear();
            q.push_str(text);
        });
        self.edit(text);
    }

    pub fn move_cursor(self, delta: i32) {
        let len = self.filtered().len();
        if len == 0 {
            self.cursor.set(0);
            return;
        }
        let current = self.cursor.peek();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            current
                .saturating_add(usize::try_from(delta).unwrap_or_default())
                .min(len - 1)
        };
        self.cursor.set(next);
    }

    pub fn commit(self, index: usize) {
        let Some(label) = self.options.peek().get(index).cloned() else {
            return;
        };
        self.selected.set(Some(index));
        self.query.set(label);
        self.open.set(false);
        self.session.update(|s| *s += 1);
    }

    pub fn commit_cursor(self) -> bool {
        let Some(&(index, _)) = self.filtered().get(self.cursor.peek()) else {
            return false;
        };
        self.commit(index);
        true
    }

    fn edit(self, text: &str) {
        self.open.set(true);
        self.cursor.set(0);
        let matches_selection = self
            .selected
            .peek()
            .and_then(|index| self.options.peek().get(index).cloned())
            .is_some_and(|label| label == text);
        if !matches_selection {
            self.selected.set(None);
        }
    }
}

#[must_use]
pub fn combobox(state: Combobox) -> NodeBuilder {
    let label = if state.placeholder.is_empty() {
        "Combobox"
    } else {
        state.placeholder
    };
    container()
        .role(Role::ComboBox)
        .label(label)
        .layout(LayoutStyle::block().fill_width().height(FIELD_HEIGHT))
        .key(Key::Escape, move |ev| {
            ev.stop_propagation();
            state.close();
        })
        .key(Key::ArrowDown, move |ev| {
            ev.stop_propagation();
            if state.open.peek() {
                state.move_cursor(1);
            } else {
                state.open();
            }
        })
        .key(Key::ArrowUp, move |ev| {
            ev.stop_propagation();
            state.move_cursor(-1);
        })
        .child(field_host(state))
        .child(popup_host(state))
}

fn field_host(state: Combobox) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fill_width())
        .children_reactive(move || {
            let _ = state.session.get();
            vec![build_field(state)]
        })
}

fn build_field(state: Combobox) -> TextFieldBuilder {
    text_field(state.query)
        .placeholder(state.placeholder)
        .height(FIELD_HEIGHT)
        .change(move |text| state.edit(text))
        .submit(move |_| {
            if state.open.peek() {
                state.commit_cursor();
            }
        })
}

fn popup_host(state: Combobox) -> NodeBuilder {
    container().children_reactive(move || {
        let matches = state.filtered();
        if state.open.get() && !matches.is_empty() {
            vec![build_popup(state, matches)]
        } else {
            Vec::new()
        }
    })
}

fn build_popup(state: Combobox, matches: Vec<(usize, String)>) -> NodeBuilder {
    let n = matches.len();
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let extent = n as f32 * ROW_HEIGHT;
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let popup_h = (n.min(MAX_VISIBLE) as f32) * ROW_HEIGHT;

    let popup = container()
        .role(Role::ListBox)
        .with(TopLayer)
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .height(popup_h)
                .position(Position::Absolute)
                .inset_top(FIELD_HEIGHT)
                .inset_left(0.0),
        )
        .class("rounded-md")
        .class("shadow-lg")
        .class(SURFACE_HOVER);

    let mut list = container().layout(LayoutStyle::flex_column().fill_width().height(extent));
    for (row, (index, label)) in matches.into_iter().enumerate() {
        list = list.child(build_row(state, row, index, label));
    }
    if n > MAX_VISIBLE {
        popup.child(scroll(list, extent).height(popup_h))
    } else {
        popup.child(list)
    }
}

fn build_row(state: Combobox, row: usize, index: usize, label: String) -> NodeBuilder {
    container()
        .role(Role::ListBoxOption)
        .label(label.clone())
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(ROW_HEIGHT)
                .padding(8.0)
                .overflow_x(Overflow::Hidden),
        )
        .style_reactive(move || row_atoms(state, row, index))
        .pointer_up(move |ev| {
            if ev.is_captured() {
                return;
            }
            state.commit(index);
            ev.stop_propagation();
        })
        .pointer_enter(move |_| {
            state.cursor.set(row);
        })
        .child(
            text(label)
                .layout(LayoutStyle::block().fill_width())
                .class("text-on-surface"),
        )
}

fn row_atoms(state: Combobox, row: usize, index: usize) -> &'static [StyleAtom] {
    if state.selected.get() == Some(index) {
        OPTION_ACTIVE
    } else if state.cursor.get() == row {
        SURFACE_HOVER
    } else {
        SURFACE
    }
}

#[cfg(test)]
#[path = "combobox_tests.rs"]
mod tests;
