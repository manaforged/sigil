use crate::kernel::{IntoNode, NodeBuilder, Role, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::style::{ControlSize, NodeStyleExt, PaintTarget, StyleSlot};
use crate::style::{StyleAtom, Tone};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

pub struct DotBuilder {
    tone: Tone,
    size: ControlSize,
    style_accumulator: crate::style::Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn dot(tone: Tone) -> DotBuilder {
    DotBuilder::new(tone)
}

impl DotBuilder {
    #[must_use]
    pub fn new(tone: Tone) -> Self {
        Self {
            tone,
            size: ControlSize::Compact,
            style_accumulator: crate::style::Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub const fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let (layout, mut node) =
            CellForm::<fn(Tone, ControlSize) -> (LayoutStyle, NodeBuilder)>::new(
                round_dot, cell_dot,
            )
            .pick()(self.tone, self.size);
        node = node
            .role(Role::GenericContainer)
            .with(self.layout_overrides.apply(layout));
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node
    }
}

fn cell_dot(tone: Tone, _size: ControlSize) -> (LayoutStyle, NodeBuilder) {
    let layout = LayoutStyle::block().width(cols(1.0)).height(rows(1.0));
    let node = container()
        .tone(tone)
        .class(DOT_GLYPH_ATOMS)
        .child(text("●"));
    (layout, node)
}

fn round_dot(tone: Tone, size: ControlSize) -> (LayoutStyle, NodeBuilder) {
    let extent: i16 = match size {
        ControlSize::Compact => 8,
        ControlSize::Default => 12,
        ControlSize::Wide | ControlSize::Grow => 16,
    };
    let (outer, inner) = (f32::from(extent), f32::from(extent - 2));
    let node = container()
        .style(crate::style::Style::new().radius(extent / 2))
        .tone(tone)
        .class(DOT_RING_ATOMS)
        .child(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(inner, inner))
                .style(crate::style::Style::new().radius((extent - 2) / 2))
                .tone(tone)
                .class(DOT_FILL_ATOMS),
        );
    let layout = LayoutStyle::flex_row()
        .fixed_size(outer, outer)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center);
    (layout, node)
}

static DOT_GLYPH_ATOMS: &[StyleAtom] = &[StyleAtom::FgToneStep(9)];
static DOT_FILL_ATOMS: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];
static DOT_RING_ATOMS: &[StyleAtom] = &[StyleAtom::BgToneStep(11)];

impl PaintTarget for DotBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for DotBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for DotBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

#[cfg(test)]
#[path = "dot_tests.rs"]
mod tests;
