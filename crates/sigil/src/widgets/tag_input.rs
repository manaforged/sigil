use std::fmt::Display;

use crate::events::Key;
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::{AlignItems, FlexWrap, LayoutStyle, Space};
use crate::reactive::{Reactive, Signal, effect};
use crate::style::Tone;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::ActiveTheme;
use crate::widgets::layout_ext::LayoutOverrides;
use crate::widgets::{Layout, Widget};

use super::button::button;
use super::chip::chip;
use super::combobox::{Combobox, combobox};
use super::icon::DISMISS_GLYPH;

/// Builder of a tag input.
#[must_use]
pub struct TagInputBuilder<T: 'static> {
    options: Vec<T>,
    selected: Signal<Vec<T>>,
    state: Combobox,
    layout_overrides: LayoutOverrides,
}

impl<T: 'static> core::fmt::Debug for TagInputBuilder<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TagInputBuilder")
            .field("options", &self.options.len())
            .field("state", &self.state)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A text input that filters `options` and adds each chosen one to `selected` as a removable tag.
///
/// Enter or a click adds the highlighted option, choosing a selected option again removes it, and Backspace in an empty input removes the last tag.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::tag_input;
///
/// fn labels() -> impl IntoNode {
///     let chosen = signal(vec!["bug".to_owned()]);
///     field("Labels", tag_input(["bug", "docs", "feature"].map(String::from), chosen))
/// }
/// ```
pub fn tag_input<T>(
    options: impl IntoIterator<Item = T>,
    selected: Signal<Vec<T>>,
) -> TagInputBuilder<T>
where
    T: Clone + PartialEq + Display + 'static,
{
    let options: Vec<T> = options.into_iter().collect();
    TagInputBuilder {
        state: Combobox::new(options.iter().map(ToString::to_string).collect()),
        options,
        selected,
        layout_overrides: LayoutOverrides::new(),
    }
}

impl<T: Clone + PartialEq + Display + 'static> TagInputBuilder<T> {
    /// Sets the placeholder.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.state = self.state.placeholder(text);
        self
    }

    /// Sets the disabled.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.state = self.state.disabled(disabled);
        self
    }

    fn root(self) -> NodeBuilder {
        let Self {
            options,
            selected,
            state,
            ..
        } = self;
        let (placeholder, disabled) = (state.placeholder_text(), state.disabled_flag());
        let picked = state.selected_signal();
        effect(move || {
            let Some(option) = picked.get().and_then(|index| options.get(index)) else {
                return;
            };
            if state
                .selected_option()
                .is_some_and(|label| label == state.query())
            {
                toggle(selected, option);
                state.edit_query("");
            }
        });
        let input = combobox(state).key_capture(Key::Backspace, move |_| {
            if state.query().is_empty() && !disabled.is_some_and(|flag| flag.peek()) {
                selected.update(|tags| {
                    tags.pop();
                });
            }
        });
        let group = container().role(Role::Group);
        let group = if placeholder.peek().is_empty() {
            group
        } else {
            group.label(placeholder)
        };
        group
            .layout(LayoutStyle::flex_column().fill_width().gap(Space::Xs))
            .child(tags(selected, disabled))
            .child(input)
    }
}

fn toggle<T: Clone + PartialEq + 'static>(selected: Signal<Vec<T>>, option: &T) {
    selected.update(|tags| match tags.iter().position(|tag| tag == option) {
        Some(at) => {
            tags.remove(at);
        }
        None => tags.push(option.clone()),
    });
}

fn tags<T>(selected: Signal<Vec<T>>, disabled: Option<Signal<bool>>) -> NodeBuilder
where
    T: Clone + PartialEq + Display + 'static,
{
    let theme = ActiveTheme::capture();
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .flex_wrap(FlexWrap::Wrap)
                .align_items(AlignItems::Center)
                .gap(Space::Xs),
        )
        .visible_when(move || !selected.get().is_empty())
        .children_reactive(move || {
            selected
                .get()
                .into_iter()
                .map(|tag| {
                    let label = tag.to_string();
                    let remove = button(DISMISS_GLYPH)
                        .ghost()
                        .square()
                        .activate(move || toggle(selected, &tag))
                        .accessible(theme.fill(StringKey::RemoveItem, &[&label]));
                    let remove = match disabled {
                        Some(flag) => remove.disabled(flag),
                        None => remove,
                    };
                    container()
                        .role(Role::ListItem)
                        .layout(
                            LayoutStyle::flex_row()
                                .align_items(AlignItems::Center)
                                .gap(Space::Xxs),
                        )
                        .child(chip(label, Tone::Neutral))
                        .child(remove)
                })
                .collect::<Vec<_>>()
        })
}

builder_impls!([T: Clone + PartialEq + Display + 'static] TagInputBuilder<T>: widget, node);
