use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::Space;
use crate::layout::snap::{self, CellForm};
use crate::layout::{AlignItems, IntoPxExt as _, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{BORDER_WIDTH, Radius, Style, StyleAtom};
use crate::widgets::Widget;

use super::button::button;
use super::layout_ext::LayoutOverrides;
use super::shared::caller_style;

static SEGMENTED_FRAME: &[StyleAtom] = &[
    StyleAtom::BorderWidth(BORDER_WIDTH),
    StyleAtom::BorderToneStep(6),
];

const SEGMENTED_HEIGHT: f32 = 32.0;

/// Builder of a segmented control.
#[derive(Debug)]
#[must_use]
pub struct SegmentedBuilder {
    active: Signal<usize>,
    labels: Vec<String>,
    name: Option<String>,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

/// A framed row of options where one is active; `.label()` names the group and `.height()` (`Widget`) sets its height.
pub fn segmented(
    active: Signal<usize>,
    labels: impl IntoIterator<Item = impl Into<String>>,
) -> SegmentedBuilder {
    SegmentedBuilder {
        active,
        labels: labels.into_iter().map(Into::into).collect(),
        name: None,
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl SegmentedBuilder {
    /// Sets the accessible name of the group.
    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn root(self) -> NodeBuilder {
        let active = self.active;
        let labels = self.labels;
        let height = self.layout_overrides.height_hint().unwrap_or_else(|| {
            CellForm::new(SEGMENTED_HEIGHT, crate::layout::rows(3.0).into_px()).pick()
        });
        let (framed, inner) = match snap::cell() {
            Some(cell) if height <= cell => (false, height),
            Some(cell) => (true, 2.0_f32.mul_add(-cell, height).max(cell)),
            None => (true, (height - 6.0).max(16.0)),
        };
        let mut node = container()
            .role(Role::RadioGroup)
            .with(
                LayoutStyle::flex_row()
                    .height(height)
                    .shrink(0.0)
                    .gap(Space::Xxs)
                    .padding(Space::px(snap::detail(2.0)))
                    .align_items(AlignItems::Center),
            )
            .class(Radius::Lg.atom());
        if framed {
            node = node.class(SEGMENTED_FRAME);
        }
        node = node.children_reactive(move || {
            let current = active.get();
            labels
                .iter()
                .enumerate()
                .map(|(index, text)| segment(active, index, text, index == current, inner))
                .collect::<Vec<_>>()
        });
        if let Some(name) = self.name {
            node = node.label(name);
        }
        caller_style(node, self.style_accumulator)
    }
}

fn segment(
    active: Signal<usize>,
    index: usize,
    label: &str,
    selected: bool,
    height: f32,
) -> NodeBuilder {
    let choice = button(label.to_owned())
        .accessible(label.to_owned())
        .height(height)
        .activate(move || active.set(index));
    let choice = if selected {
        choice.primary()
    } else {
        choice.ghost()
    };
    container().child(choice)
}

builder_impls!([] SegmentedBuilder: paint(style_accumulator), states(style_accumulator), widget, node);
