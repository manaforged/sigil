use crate::layout::Space;
use crate::reactive::Reactive;

use crate::kernel::{NodeBuilder, NodeId, Role, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, cols, rows};
use crate::reactive::{Signal, memo};
use crate::style::{ControlSize, Metrics, Style, StyleAtom, Token, Tone};
use crate::widgets::Widget;

use super::layout_ext::LayoutOverrides;
use super::listbox::{Move, RADIO_GROUP};
use super::shared::{caller_style, control, disabled_a11y, disabled_now};

/// Builder of a radio group.
#[derive(Debug)]
#[must_use]
pub struct RadioGroupBuilder {
    selected: Signal<usize>,
    options: Vec<String>,
    tone: Tone,
    disabled: Option<Signal<bool>>,
    labelled_by: Option<NodeId>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// A set of radio options that reads and writes the index `selected` into `options`; `.accessible()` (`Widget`) and `.labelled_by()` name the group.
pub fn radio_group<I, S>(selected: Signal<usize>, options: I) -> RadioGroupBuilder
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    RadioGroupBuilder {
        selected,
        options: options.into_iter().map(Into::into).collect(),
        tone: Tone::Primary,
        disabled: None,
        labelled_by: None,
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl RadioGroupBuilder {
    /// Sets the color of the selected mark; the default is `Tone::Primary`.
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Disables every option while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Takes the accessible name of the group from the node `node`.
    pub const fn labelled_by(mut self, node: NodeId) -> Self {
        self.labelled_by = Some(node);
        self
    }

    fn root(self) -> NodeBuilder {
        let selected = self.selected;
        let disabled = self.disabled;
        let count = self.options.len();
        let group_focus = crate::reactive::signal(false);
        let mut group = container()
            .role(Role::RadioGroup)
            .focusable(true)
            .with(
                self.layout_overrides
                    .apply(LayoutStyle::flex_column().gap(Space::Xs)),
            )
            .focus_in(move || group_focus.set(true))
            .focus_out(move || group_focus.set(false));
        if let Some(node) = self.labelled_by {
            group = group.labelled_by(node);
        }
        let ring = memo(move || {
            group_focus
                .get()
                .then(|| focus_index(selected.get(), count))
        });
        for (index, label) in self.options.iter().enumerate() {
            group = group.child(option_row(
                index,
                label,
                selected,
                (self.tone, disabled),
                ring,
            ));
        }
        let mut group = disabled_a11y(caller_style(group, self.style_accumulator), disabled);
        for (key, step) in RADIO_GROUP.bindings() {
            group = group.key(key, move |event| {
                if !disabled_now(disabled) {
                    select(selected, count, step);
                }
                event.stop_propagation();
            });
        }
        group
    }
}

const fn focus_index(selected: usize, count: usize) -> usize {
    if selected < count { selected } else { 0 }
}

fn select(selected: Signal<usize>, count: usize, step: Move) {
    let from = focus_index(selected.peek(), count);
    if let Some(index) = RADIO_GROUP.target(&count, from, step) {
        selected.set(index);
    }
}

fn option_row(
    index: usize,
    label: &str,
    selected: Signal<usize>,
    (tone, disabled): (Tone, Option<Signal<bool>>),
    ring: Signal<Option<usize>>,
) -> NodeBuilder {
    let checked: Signal<bool> = memo(move || selected.get() == index);
    let ring_visible = memo(move || ring.get() == Some(index));
    let row = container().layout(
        LayoutStyle::flex_row()
            .height(Metrics::current().control_height(ControlSize::Default))
            .padding_x(Space::Xs)
            .gap(Space::Sm)
            .align_items(AlignItems::Center),
    );
    let mut row = control(row, disabled)
        .role(Role::RadioButton)
        .accessible(label.to_owned())
        .checked(checked)
        .tone_scope(tone)
        .focus_ring(ring_visible)
        .child(CellForm::<fn(Signal<bool>, Tone) -> NodeBuilder>::new(
            option_mark,
            cell_mark,
        )
        .pick()(checked, tone))
        .child(text(label).class(Token::OnSurface.text()));
    let interaction = row.interaction();
    row.pointer_up(move |event| {
        if interaction.release_pointer(event) {
            selected.set(index);
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
        .class(Tone::Neutral.step(7).background())
        .style_reactive(move || selected_atoms(checked))
        .child(mark_core)
}

fn cell_mark(checked: Signal<bool>, tone: Tone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().width(cols(1.0)).height(rows(1.0)))
        .tone_scope(tone)
        .class(CELL_MARK)
        .child(text(memo(move || {
            if checked.get() { "●" } else { "○" }.to_owned()
        })))
}

static CELL_MARK: &[StyleAtom] = &[StyleAtom::FgToneStep(9)];
static MARK_SELECTED: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];
static MARK_EMPTY: &[StyleAtom] = &[];

builder_impls!([] RadioGroupBuilder: paint(style_accumulator), states(style_accumulator), widget, node);

#[cfg(test)]
#[path = "radio_group_tests.rs"]
mod tests;
