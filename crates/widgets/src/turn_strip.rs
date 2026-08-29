use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{FocusRing, NodeStyleExt, Style, StyleAccess};

use sigil_style::Tone;

use super::chip::chip;

const STRIP_GAP: f32 = 6.0;

const ACTIVE_CLASS: &str = "bg-accent text-on-accent";

pub struct TurnStripBuilder {
    entries: Signal<Vec<String>>,
    active: Option<Signal<usize>>,
    style_accumulator: Style,
}

#[must_use]
pub fn turn_strip(entries: Signal<Vec<String>>) -> TurnStripBuilder {
    TurnStripBuilder::new(entries)
}

impl TurnStripBuilder {
    #[must_use]
    pub fn new(entries: Signal<Vec<String>>) -> Self {
        Self {
            entries,
            active: None,
            style_accumulator: Style::new(),
        }
    }

    #[must_use]
    pub const fn active(mut self, index: Signal<usize>) -> Self {
        self.active = Some(index);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let entries = self.entries;
        let active = self.active;
        let style_accumulator = self.style_accumulator;

        let mut node = container()
            .role(Role::GenericContainer)
            .layout(strip_layout())
            .children_reactive(move || {
                entries
                    .get()
                    .into_iter()
                    .enumerate()
                    .map(|(idx, label)| chip_entry(label, idx, active))
                    .collect::<Vec<_>>()
            });

        if !style_accumulator.is_empty() {
            node = node.style(style_accumulator);
        }
        node
    }
}

impl StyleAccess for TurnStripBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for TurnStripBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn strip_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .gap(STRIP_GAP)
        .align_items(AlignItems::Center)
}

fn chip_entry(label: String, idx: usize, active: Option<Signal<usize>>) -> NodeBuilder {
    let is_active = derived_eq(move || active.is_some_and(|active| active.get() == idx));
    chip(label, Tone::Neutral)
        .into_node_builder()
        .class_when(is_active, ACTIVE_CLASS)
        .focus_ring(FocusRing::new(is_active))
}
