use crate::layout::Space;
use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{AccessibleName, NodeBuilder, NodeId, Role, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, cols, rows};
use crate::reactive::{Signal, derived};
use crate::style::{ControlSize, Metrics, Style, StyleAtom, Token, Tone};
use crate::widgets::{Layout, Widget};

use super::layout_ext::LayoutOverrides;
use super::listbox::{Move, RADIO_GROUP};
use super::shared::{caller_style, control, disabled_a11y, disabled_now};

/// Builder of a radio group.
#[derive(Debug)]
pub struct RadioGroupBuilder {
    selected: Signal<String>,
    options: Vec<(String, String)>,
    tone: Tone,
    disabled: Option<Signal<bool>>,
    name: AccessibleName,
    labelled_by: Option<NodeId>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// A set of radio options; `.label()` and `.labelled_by()` name the group.
#[must_use]
pub fn radio_group(selected: Signal<String>) -> RadioGroupBuilder {
    RadioGroupBuilder::new(selected)
}

impl RadioGroupBuilder {
    /// Creates a radio group builder.
    #[must_use]
    pub fn new(selected: Signal<String>) -> Self {
        Self {
            selected,
            options: Vec::new(),
            tone: Tone::Primary,
            disabled: None,
            name: AccessibleName::None,
            labelled_by: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Sets the option.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push((key.into(), label.into()));
        self
    }

    /// Sets the tone.
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, name: impl Into<AccessibleName>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the labelled by.
    #[must_use]
    pub const fn labelled_by(mut self, node: NodeId) -> Self {
        self.labelled_by = Some(node);
        self
    }

    fn root(self) -> NodeBuilder {
        let selected = self.selected;
        let disabled = self.disabled;
        let keys: Rc<[String]> = self.options.iter().map(|(key, _)| key.clone()).collect();
        let group_focus = crate::reactive::signal(false);
        let mut group = container()
            .role(Role::RadioGroup)
            .focusable(true)
            .with(
                self.layout_overrides
                    .apply(LayoutStyle::flex_column().gap(Space::Xs)),
            )
            .focus_in(move || group_focus.set(true))
            .focus_out(move || group_focus.set(false))
            .label(self.name);
        if let Some(node) = self.labelled_by {
            group = group.labelled_by(node);
        }
        for (key, label) in self.options {
            group = group.child(option_row(
                key,
                &label,
                selected,
                (self.tone, disabled),
                group_focus,
            ));
        }
        let mut group = disabled_a11y(caller_style(group, self.style_accumulator), disabled);
        for (key, step) in RADIO_GROUP.bindings() {
            let keys = Rc::clone(&keys);
            group = group.key(key, move |event| {
                if !disabled_now(disabled) {
                    select(selected, &keys, step);
                }
                event.stop_propagation();
            });
        }
        group
    }
}

fn select(selected: Signal<String>, keys: &[String], step: Move) {
    let current = selected.get();
    let from = keys.iter().position(|key| key == &current).unwrap_or(0);
    let target = RADIO_GROUP.target(&keys.len(), from, step);
    if let Some(key) = target.and_then(|index| keys.get(index)) {
        selected.set(key.clone());
    }
}

fn option_row(
    key: String,
    label: &str,
    selected: Signal<String>,
    (tone, disabled): (Tone, Option<Signal<bool>>),
    group_focus: Signal<bool>,
) -> NodeBuilder {
    let checked_key = key.clone();
    let checked: Signal<bool> = derived(move || selected.get() == checked_key);
    let ring_visible = derived(move || group_focus.get() && checked.get());
    let row = container().layout(
        LayoutStyle::flex_row()
            .height(Metrics::current().control_height(ControlSize::Default))
            .padding_x(Space::Xs)
            .gap(Space::Sm)
            .align_items(AlignItems::Center),
    );
    let activate_key = key;
    control(row, disabled)
        .role(Role::RadioButton)
        .label(label.to_owned())
        .checked(checked)
        .tone_scope(tone)
        .focus_ring(ring_visible)
        .child(CellForm::<fn(Signal<bool>, Tone) -> NodeBuilder>::new(
            option_mark,
            cell_mark,
        )
        .pick()(checked, tone))
        .child(text(label).class(Token::OnSurface.text()))
        .pointer_up(move |_event| selected.set(activate_key.clone()))
}

fn centered_square(size: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(size, size)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn selected_atoms(checked: Signal<bool>) -> &'static [StyleAtom] {
    if checked.get() {
        MARK_SELECTED
    } else {
        MARK_EMPTY
    }
}

const MARK_OUTER: f32 = 16.0;
const MARK_CORE: f32 = 13.0;
const MARK_INNER: f32 = 8.0;

fn option_mark(checked: Signal<bool>, tone: Tone) -> NodeBuilder {
    let mark_inner = container()
        .layout(LayoutStyle::flex_row().fixed_size(MARK_INNER, MARK_INNER))
        .tone_scope(tone)
        .style(Style::new().radius_px(4))
        .style_reactive(move || selected_atoms(checked));
    let mark_core = container()
        .layout(centered_square(MARK_CORE))
        .style(Style::new().radius_px(7))
        .class(Token::Surface.background())
        .child(mark_inner);
    container()
        .role(Role::GenericContainer)
        .layout(centered_square(MARK_OUTER))
        .style(Style::new().radius_px(8))
        .tone_scope(tone)
        .class(Token::Gray7.background())
        .style_reactive(move || selected_atoms(checked))
        .child(mark_core)
}

fn cell_mark(checked: Signal<bool>, tone: Tone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().width(cols(1.0)).height(rows(1.0)))
        .tone_scope(tone)
        .class(CELL_MARK)
        .child(text(derived(move || {
            if checked.get() { "●" } else { "○" }.to_owned()
        })))
}

static CELL_MARK: &[StyleAtom] = &[StyleAtom::FgToneStep(9)];
static MARK_SELECTED: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];
static MARK_EMPTY: &[StyleAtom] = &[];

builder_impls!([] RadioGroupBuilder: paint(style_accumulator), widget, node);

#[cfg(test)]
#[path = "radio_group_tests.rs"]
mod tests;
