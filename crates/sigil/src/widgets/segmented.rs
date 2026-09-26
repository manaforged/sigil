use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{BORDER_WIDTH, NodeStyleExt, Radius, StyleAtom};

use super::button::button;
use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

static SEGMENTED_FRAME: &[StyleAtom] = &[
    StyleAtom::BorderWidth(BORDER_WIDTH),
    StyleAtom::BorderToneStep(6),
];

pub struct SegmentedBuilder {
    active: Signal<usize>,
    labels: Vec<String>,
    name: Option<String>,
    height: f32,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub const fn segmented(active: Signal<usize>, labels: Vec<String>) -> SegmentedBuilder {
    SegmentedBuilder {
        active,
        labels,
        name: None,
        height: 32.0,
        layout_overrides: LayoutOverrides::new(),
    }
}

impl SegmentedBuilder {
    #[must_use]
    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let active = self.active;
        let labels = self.labels;
        let inner = (self.height - 6.0).max(16.0);
        let mut node = container()
            .role(Role::RadioGroup)
            .layout(
                self.layout_overrides.apply(
                    LayoutStyle::flex_row()
                        .height(self.height)
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
                    .map(|(index, text)| {
                        let choice = button(text.clone())
                            .height(inner)
                            .activate(move || active.set(index));
                        let choice = if index == current {
                            choice.primary()
                        } else {
                            choice.ghost()
                        };
                        container().child(choice)
                    })
                    .collect::<Vec<_>>()
            });
        if let Some(name) = self.name {
            node = node.label(name);
        }
        node
    }
}

impl WidgetLayoutExt for SegmentedBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for SegmentedBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}
