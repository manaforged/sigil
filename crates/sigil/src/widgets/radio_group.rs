use crate::reactive::Reactive;
use std::rc::Rc;

use crate::events::{Key, NodeEventsExt};
use crate::kernel::{AccessibleName, NodeBuilder, NodeId, Role, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::reactive::{Signal, derived};
use crate::style::{
    Metrics, NodeStyleExt, Size, Style, StyleAtom, StyleOverlay, Token, Tone, space,
};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{control_surface, disabled_a11y, disabled_now};
use crate::kernel::IntoNode;
use crate::style::StyleAccess;
use crate::widgets::interaction::InteractionState;

type Locked = Option<Signal<bool>>;

#[derive(Clone, Debug)]
struct RadioOption {
    key: String,
    label: String,
}

pub struct RadioGroupBuilder {
    selected: Signal<String>,
    options: Vec<RadioOption>,
    tone: Tone,
    disabled: Option<Signal<bool>>,
    name: AccessibleName,
    labelled_by: Option<NodeId>,
    style_accumulator: crate::style::Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn radio_group(selected: Signal<String>) -> RadioGroupBuilder {
    RadioGroupBuilder::new(selected)
}

impl RadioGroupBuilder {
    #[must_use]
    pub fn new(selected: Signal<String>) -> Self {
        Self {
            selected,
            options: Vec::new(),
            tone: Tone::Primary,
            disabled: None,
            name: AccessibleName::None,
            labelled_by: None,
            style_accumulator: crate::style::Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub fn option(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(RadioOption {
            key: key.into(),
            label: label.into(),
        });
        self
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn label(mut self, name: impl Into<AccessibleName>) -> Self {
        self.name = name.into();
        self
    }

    #[must_use]
    pub const fn labelled_by(mut self, node: NodeId) -> Self {
        self.labelled_by = Some(node);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let selected = self.selected;
        let tone = self.tone;
        let disabled = self.disabled;
        let keys: Vec<String> = self.options.iter().map(|opt| opt.key.clone()).collect();
        let group_focus = crate::reactive::signal(false);
        let mut group = container()
            .role(Role::RadioGroup)
            .focusable(true)
            .with(self.layout_overrides.apply(group_layout()))
            .focus_in(move || group_focus.set(true))
            .focus_out(move || group_focus.set(false))
            .label(self.name);
        if let Some(node) = self.labelled_by {
            group = group.labelled_by(node);
        }
        for opt in self.options {
            group = group.child(option_row(opt, selected, (tone, disabled), group_focus));
        }
        if !self.style_accumulator.is_empty() {
            group = group.style(self.style_accumulator);
        }
        let next_keys = keys.clone();
        let prev_keys = keys.clone();
        let home_keys = keys.clone();
        let end_keys = keys;
        disabled_a11y(group, disabled)
            .key(Key::ArrowDown, move |event| {
                select_offset(selected, &next_keys, 1, disabled);
                event.stop_propagation();
            })
            .key(Key::ArrowUp, move |event| {
                select_offset(selected, &prev_keys, -1, disabled);
                event.stop_propagation();
            })
            .key(Key::Home, move |event| {
                select_edge(selected, &home_keys, true, disabled);
                event.stop_propagation();
            })
            .key(Key::End, move |event| {
                select_edge(selected, &end_keys, false, disabled);
                event.stop_propagation();
            })
    }
}

fn select_offset(selected: Signal<String>, keys: &[String], delta: isize, disabled: Locked) {
    if keys.is_empty() || disabled_now(disabled) {
        return;
    }
    let current = selected.get();
    let index = keys
        .iter()
        .position(|key| key == &current)
        .map_or(0, usize::cast_signed);
    let len = keys.len().cast_signed();
    let next = (index + delta).rem_euclid(len).cast_unsigned();
    if let Some(key) = keys.get(next) {
        selected.set(key.clone());
    }
}

fn select_edge(selected: Signal<String>, keys: &[String], first: bool, disabled: Locked) {
    if disabled_now(disabled) {
        return;
    }
    let key = if first { keys.first() } else { keys.last() };
    if let Some(key) = key {
        selected.set(key.clone());
    }
}

fn option_row(
    opt: RadioOption,
    selected: Signal<String>,
    (tone, disabled): (Tone, Option<Signal<bool>>),
    group_focus: Signal<bool>,
) -> NodeBuilder {
    let RadioOption { key, label } = opt;
    let key_for_activate = key.clone();
    let activate = Rc::new(move || selected.set(key_for_activate.clone()));
    let activate_for_pointer = Rc::clone(&activate);
    let key_for_checked = key.clone();
    let checked: Signal<bool> = derived(move || selected.get() == key_for_checked);
    let key_for_ring = key.clone();
    let ring_visible = derived(move || group_focus.get() && selected.get() == key_for_ring);
    let interaction = InteractionState::new(true);
    let row = interaction.observe_style(container().layout(option_layout()));
    control_surface(row, interaction, disabled, StyleOverlay::default())
        .role(Role::RadioButton)
        .label(label.clone())
        .checked(checked)
        .tone(tone)
        .focus_ring(crate::style::FocusRing::new(ring_visible))
        .child(
            CellForm::<fn(Signal<String>, String, Tone) -> NodeBuilder>::new(
                option_mark,
                cell_mark,
            )
            .pick()(selected, key, tone),
        )
        .child(text(label).class(Token::OnSurface.text()))
        .pointer_up(move |_event| {
            if !disabled_now(disabled) {
                activate_for_pointer();
            }
        })
}

fn option_mark(selected: Signal<String>, key: String, tone: Tone) -> NodeBuilder {
    let ring_key = key.clone();
    let mark_inner = container()
        .layout(mark_inner_layout())
        .tone(tone)
        .style(Style::new().radius(4))
        .style_reactive(move || {
            if selected.get() == key {
                MARK_SELECTED
            } else {
                MARK_EMPTY
            }
        });
    let mark_core = container()
        .layout(mark_core_layout())
        .style(Style::new().radius(7))
        .class(Token::Surface.bg())
        .child(mark_inner);
    container()
        .role(Role::GenericContainer)
        .layout(mark_outer_layout())
        .style(Style::new().radius(8))
        .tone(tone)
        .class(Token::Gray7.bg())
        .style_reactive(move || {
            if selected.get() == ring_key {
                RING_SELECTED
            } else {
                MARK_EMPTY
            }
        })
        .child(mark_core)
}

fn cell_mark(selected: Signal<String>, key: String, tone: Tone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().width(cols(1.0)).height(rows(1.0)))
        .tone(tone)
        .class(CELL_MARK)
        .child(text(derived(move || {
            if selected.get() == key { "●" } else { "○" }.to_owned()
        })))
}

static CELL_MARK: &[StyleAtom] = &[StyleAtom::FgToneStep(9)];
static MARK_SELECTED: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];
static RING_SELECTED: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];
static MARK_EMPTY: &[StyleAtom] = &[];

const MARK_OUTER: f32 = 16.0;
const MARK_CORE: f32 = 13.0;
const MARK_INNER: f32 = 8.0;

fn group_layout() -> LayoutStyle {
    LayoutStyle::flex_column().gap(space(1))
}

fn option_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .height(Metrics::current().control_height(Size::Default))
        .padding_x(space(1))
        .gap(space(2))
        .align_items(AlignItems::Center)
}

fn mark_outer_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(MARK_OUTER, MARK_OUTER)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn mark_core_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(MARK_CORE, MARK_CORE)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn mark_inner_layout() -> LayoutStyle {
    LayoutStyle::flex_row().fixed_size(MARK_INNER, MARK_INNER)
}

impl StyleAccess for RadioGroupBuilder {
    fn style_mut(&mut self) -> &mut crate::style::Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for RadioGroupBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for RadioGroupBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[path = "radio_group_tests.rs"]
mod tests;
