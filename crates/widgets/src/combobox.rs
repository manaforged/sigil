//! Combobox — searchable select: a text field with a filtered popup list
//! (Radix-style), for picking one option out of a long list.
//!
//! [`Combobox`] is a `Copy` handle backed by signals: the option list, the
//! current query, the selection, and the popup open state. [`combobox`]
//! renders a [`text_field`](crate::text_field) with a popup below it that
//! lists options filtered case-insensitively by the query substring.
//! Pointer click and keyboard (Up/Down/Enter) commit a selection; Escape
//! closes the popup; clearing the text clears the selection.
//!
//! The popup reuses the dropdown's primitives (absolute top-layer listbox,
//! option rows, scroll past [`MAX_VISIBLE`] rows) so behavior and styling
//! stay consistent.
//!
//! ```ignore
//! use sigil::prelude::*;
//!
//! let state = Combobox::new(accounts).placeholder("Search accounts…");
//! // in the tree: combobox(state)
//! // react to selection: state.selected_signal()
//! ```
//!
//! Known gap: committing a selection rebuilds the inner text field (the
//! field's edit buffer only seeds at build, so the committed label would
//! otherwise go stale on the next keystroke — the same limitation
//! `number_field` documents). The rebuild drops pointer/keyboard focus;
//! the user re-focuses with a click or Tab.

use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{NodeBuilder, Role, TopLayer, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Overflow, Position};
use sigil_reactive::{Signal, signal};
use sigil_style::{NodeStyleExt, StyleAtom};

use super::layout_ext::WidgetLayoutExt;
use super::scroll::scroll;
use super::text_field::{TextFieldBuilder, text_field};

/// Maximum number of option rows visible before the popup scrolls.
const MAX_VISIBLE: usize = 8;
/// Option row height in logical pixels (matches the dropdown's).
const ROW_HEIGHT: f32 = 28.0;
/// Field height; the popup pins its top edge to the field's bottom.
const FIELD_HEIGHT: f32 = 32.0;

static SURFACE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static SURFACE_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static OPTION_ACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

/// Clone handle to a combobox's state. Signal-backed: reads and writes
/// from any clone re-render every [`combobox`] widget bound to the state.
///
/// Must be created inside a reactive scope (it allocates signals).
#[derive(Clone, Copy)]
pub struct Combobox {
    options: Signal<Vec<String>>,
    query: Signal<String>,
    selected: Signal<Option<usize>>,
    open: Signal<bool>,
    cursor: Signal<usize>,
    /// Field-rebuild generation, bumped on commit so the inner text field
    /// re-seeds its edit buffer from the committed label.
    session: Signal<u32>,
    placeholder: &'static str,
}

impl Combobox {
    /// Create a combobox over `options` with no selection and the popup
    /// closed.
    ///
    /// # Panics
    ///
    /// Panics outside a reactive scope, same as [`sigil_reactive::signal`].
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

    /// Set the placeholder text rendered while the field is empty.
    #[must_use]
    pub const fn placeholder(mut self, text: &'static str) -> Self {
        self.placeholder = text;
        self
    }

    /// Current query text.
    pub fn query(self) -> String {
        self.query.get()
    }

    /// Index of the selected option, if any.
    pub fn selected(self) -> Option<usize> {
        self.selected.get()
    }

    /// Label of the selected option, if any.
    pub fn selected_option(self) -> Option<String> {
        self.selected
            .get()
            .and_then(|index| self.options.peek().get(index).cloned())
    }

    /// The selection as a signal, for reactive consumers.
    pub const fn selected_signal(self) -> Signal<Option<usize>> {
        self.selected
    }

    /// Whether the popup is open.
    pub fn is_open(self) -> bool {
        self.open.get()
    }

    /// Open the popup, moving the cursor to the selected row when one is
    /// visible in the current filter.
    pub fn open(self) {
        let row = self
            .selected
            .peek()
            .and_then(|index| self.filtered().iter().position(|(i, _)| *i == index))
            .unwrap_or(0);
        self.cursor.set(row);
        self.open.set(true);
    }

    /// Close the popup.
    pub fn close(self) {
        self.open.set(false);
    }

    /// Options matching the current query as `(option index, label)`
    /// pairs — case-insensitive substring match; an empty query matches
    /// every option.
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

    /// The highlighted popup row (index into [`Combobox::filtered`]).
    pub fn cursor(self) -> usize {
        self.cursor.get()
    }

    /// Replace the query text: opens the popup, resets the cursor, and
    /// clears the selection when the new text no longer matches the
    /// selected option's label (so clearing the text clears the
    /// selection).
    pub fn edit_query(self, text: &str) {
        self.query.update(|q| {
            q.clear();
            q.push_str(text);
        });
        self.edit(text);
    }

    /// Move the cursor by `delta` rows, clamped to the filtered list.
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

    /// Commit option `index`: record the selection, write its label into
    /// the field, and close the popup.
    pub fn commit(self, index: usize) {
        let Some(label) = self.options.peek().get(index).cloned() else {
            return;
        };
        self.selected.set(Some(index));
        self.query.set(label);
        self.open.set(false);
        self.session.update(|s| *s += 1);
    }

    /// Commit the option under the cursor. Returns `false` (a no-op) when
    /// the filtered list is empty.
    pub fn commit_cursor(self) -> bool {
        let Some(&(index, _)) = self.filtered().get(self.cursor.peek()) else {
            return false;
        };
        self.commit(index);
        true
    }

    /// Post-edit bookkeeping shared by field edits and [`Combobox::edit_query`].
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

/// Render a [`Combobox`] as a text field with a filtered popup below it.
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

/// Host that rebuilds the text field only when `session` bumps (commit),
/// so per-keystroke query writes never churn the focused field node.
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

/// Host that rebuilds the popup when the open state or the query changes.
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

/// Popup listbox below the field, mirroring the dropdown's: absolute
/// top-layer surface, capped at [`MAX_VISIBLE`] rows, scrolling past that.
fn build_popup(state: Combobox, matches: Vec<(usize, String)>) -> NodeBuilder {
    let n = matches.len();
    #[allow(clippy::cast_precision_loss)]
    let extent = n as f32 * ROW_HEIGHT;
    #[allow(clippy::cast_precision_loss)]
    let popup_h = (n.min(MAX_VISIBLE) as f32) * ROW_HEIGHT;

    let popup = container()
        .role(Role::ListBox)
        // Top layer: without this the popup paints in tree order and any
        // later sibling draws over it (same as the dropdown popup).
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
                // Clip labels wider than the row at the popup's edge
                // instead of painting past it (matches the dropdown).
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
