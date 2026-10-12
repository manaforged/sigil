use crate::reactive::Reactive;
use std::rc::Rc;

use crate::events::{Key, NodeEventsExt};
use crate::kernel::{AccessibleName, NodeBuilder, NodeId, Role, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::reactive::{Signal, derived};
use crate::style::{
    ControlSize, Metrics, NodeStyleExt, Style, StyleAtom, StyleOverlay, Token, Tone, space,
};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{control_surface, disabled_a11y, disabled_now};
use crate::kernel::IntoNode;
use crate::style::{PaintTarget, StyleSlot};
use crate::widgets::interaction::InteractionState;

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
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub fn option(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push((key.into(), label.into()));
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
        let disabled = self.disabled;
        let keys: Rc<[String]> = self.options.iter().map(|(key, _)| key.clone()).collect();
        let group_focus = crate::reactive::signal(false);
        let mut group = container()
            .role(Role::RadioGroup)
            .focusable(true)
            .with(
                self.layout_overrides
                    .apply(LayoutStyle::flex_column().gap(space(1))),
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
        if !self.style_accumulator.is_empty() {
            group = group.style(self.style_accumulator);
        }
        let mut group = disabled_a11y(group, disabled);
        for nav in [Key::ArrowDown, Key::ArrowUp, Key::Home, Key::End] {
            let keys = Rc::clone(&keys);
            group = group.key(nav, move |event| {
                if !disabled_now(disabled) {
                    select(selected, &keys, nav);
                }
                event.stop_propagation();
            });
        }
        group
    }
}

fn select(selected: Signal<String>, keys: &[String], nav: Key) {
    let target = match nav {
        Key::Home => keys.first(),
        Key::End => keys.last(),
        _ if keys.is_empty() => None,
        _ => {
            let current = selected.get();
            let index = keys
                .iter()
                .position(|key| key == &current)
                .map_or(0, usize::cast_signed);
            let delta = if nav == Key::ArrowDown { 1 } else { -1 };
            keys.get(
                (index + delta)
                    .rem_euclid(keys.len().cast_signed())
                    .cast_unsigned(),
            )
        }
    };
    if let Some(key) = target {
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
    let interaction = InteractionState::new(true);
    let row = interaction.observe_style(
        container().layout(
            LayoutStyle::flex_row()
                .height(Metrics::current().control_height(ControlSize::Default))
                .padding_x(space(1))
                .gap(space(2))
                .align_items(AlignItems::Center),
        ),
    );
    let activate_key = key;
    control_surface(row, interaction, disabled, StyleOverlay::default())
        .role(Role::RadioButton)
        .label(label.to_owned())
        .checked(checked)
        .tone(tone)
        .focus_ring(crate::style::FocusRing::new(ring_visible))
        .child(CellForm::<fn(Signal<bool>, Tone) -> NodeBuilder>::new(
            option_mark,
            cell_mark,
        )
        .pick()(checked, tone))
        .child(text(label).class(Token::OnSurface.text()))
        .pointer_up(move |_event| {
            if !disabled_now(disabled) {
                selected.set(activate_key.clone());
            }
        })
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
        .tone(tone)
        .style(Style::new().radius(4))
        .style_reactive(move || selected_atoms(checked));
    let mark_core = container()
        .layout(centered_square(MARK_CORE))
        .style(Style::new().radius(7))
        .class(Token::Surface.bg())
        .child(mark_inner);
    container()
        .role(Role::GenericContainer)
        .layout(centered_square(MARK_OUTER))
        .style(Style::new().radius(8))
        .tone(tone)
        .class(Token::Gray7.bg())
        .style_reactive(move || selected_atoms(checked))
        .child(mark_core)
}

fn cell_mark(checked: Signal<bool>, tone: Tone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().width(cols(1.0)).height(rows(1.0)))
        .tone(tone)
        .class(CELL_MARK)
        .child(text(derived(move || {
            if checked.get() { "●" } else { "○" }.to_owned()
        })))
}

static CELL_MARK: &[StyleAtom] = &[StyleAtom::FgToneStep(9)];
static MARK_SELECTED: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];
static MARK_EMPTY: &[StyleAtom] = &[];

impl PaintTarget for RadioGroupBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for RadioGroupBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for RadioGroupBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

#[cfg(test)]
#[path = "radio_group_tests.rs"]
mod tests;
