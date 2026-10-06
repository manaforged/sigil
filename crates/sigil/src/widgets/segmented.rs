use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{BORDER_WIDTH, PaintTarget, Radius, Style, StyleAtom, StyleSlot};

use super::button::button;
use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

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
        let inner = (height - 6.0).max(16.0);
        let mut node = container()
            .role(Role::RadioGroup)
            .with(
                self.layout_overrides.apply(
                    LayoutStyle::flex_row()
                        .height(height)
                        .shrink(0.0)
                        .gap(2.0)
                        .padding(2.0)
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
    let shown =
        CellForm::<fn(&str, bool) -> String>::new(window_label, cell_label).pick()(label, selected);
    let choice = button(shown)
        .accessible(label)
        .height(height)
        .activate(move || active.set(index));
    let choice = if selected {
        choice.primary()
    } else {
        choice.ghost()
    };
    container().child(choice)
}

fn window_label(label: &str, _selected: bool) -> String {
    label.to_owned()
}

fn cell_label(label: &str, selected: bool) -> String {
    if selected {
        format!("{CELL_ACTIVE_OPEN}{label}{CELL_ACTIVE_CLOSE}")
    } else {
        label.to_owned()
    }
}

impl PaintTarget for SegmentedBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for SegmentedBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for SegmentedBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}
