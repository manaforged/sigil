use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::Space;
use crate::layout::snap::{self, CellForm};
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{BORDER_WIDTH, Radius, Style, StyleAtom};
use crate::widgets::Widget;

use super::button::button;
use super::layout_ext::LayoutOverrides;

static SEGMENTED_FRAME: &[StyleAtom] = &[
    StyleAtom::BorderWidth(BORDER_WIDTH),
    StyleAtom::BorderToneStep(6),
];

const SEGMENTED_HEIGHT: f32 = 32.0;

/// Builder of a segmented control.
pub struct SegmentedBuilder {
    active: Signal<usize>,
    labels: Vec<String>,
    name: Option<String>,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

/// A framed row of options where one is active; `.label()` names the group and `.height()` (`Widget`) sets its height.
#[must_use]
pub fn segmented(active: Signal<usize>, labels: Vec<String>) -> SegmentedBuilder {
    SegmentedBuilder {
        active,
        labels,
        name: None,
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl SegmentedBuilder {
    /// Sets the label.
    #[must_use]
    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn root(self) -> NodeBuilder {
        let active = self.active;
        let labels = self.labels;
        let height = self
            .layout_overrides
            .height_hint()
            .unwrap_or(SEGMENTED_HEIGHT);
        let inner = CellForm::new((height - 6.0).max(16.0), height).pick();
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
            .class(Radius::Lg.atom())
            .class(SEGMENTED_FRAME)
            .children_reactive(move || {
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
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node
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

builder_impls!([] SegmentedBuilder: paint(style_accumulator), widget, node);
