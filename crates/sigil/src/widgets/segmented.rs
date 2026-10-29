use super::layout_ext::WidgetLayoutExt;
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::snap::{self, CellForm};
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{BORDER_WIDTH, Radius, Style, StyleAtom};

use super::button::button;
use super::layout_ext::LayoutOverrides;

static SEGMENTED_FRAME: &[StyleAtom] = &[
    StyleAtom::BorderWidth(BORDER_WIDTH),
    StyleAtom::BorderToneStep(6),
];

const SEGMENTED_HEIGHT: f32 = 32.0;
const CELL_ACTIVE_OPEN: &str = "[";
const CELL_ACTIVE_CLOSE: &str = "]";

pub struct SegmentedBuilder {
    active: Signal<usize>,
    labels: Vec<String>,
    name: Option<String>,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

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
    #[must_use]
    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
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
                self.layout_overrides.apply(
                    LayoutStyle::flex_row()
                        .height(height)
                        .shrink(0.0)
                        .gap(2.0)
                        .padding(snap::detail(2.0))
                        .align_items(AlignItems::Center),
                ),
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
    let shown = if CellForm::new(false, selected).pick() {
        format!("{CELL_ACTIVE_OPEN}{label}{CELL_ACTIVE_CLOSE}")
    } else {
        label.to_owned()
    };
    let choice = button(shown)
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

builder_impls!([] SegmentedBuilder: paint(style_accumulator), layout, node);
