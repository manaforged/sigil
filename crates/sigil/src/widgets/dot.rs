use crate::kernel::{IntoNode, NodeBuilder, Role, container, text};
use crate::layout::prelude::{
    AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, cols, rows,
};
use crate::layout::snap::CellForm;
use crate::style::{NodeStyleExt, Size, StyleAccess};
use crate::style::{Px, StyleAtom, Tone};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

pub struct DotBuilder {
    tone: Tone,
    size: Size,
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
            size: Size::Compact,
            style_accumulator: crate::style::Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let (layout, mut node) =
            CellForm::<fn(Tone, Size) -> (LayoutStyle, NodeBuilder)>::new(round_dot, cell_dot)
                .pick()(self.tone, self.size);
        node = node
            .role(Role::GenericContainer)
            .layout(self.layout_overrides.apply(layout));
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node
    }
}

fn cell_dot(tone: Tone, _size: Size) -> (LayoutStyle, NodeBuilder) {
    let layout = LayoutStyle::block().width(cols(1.0)).height(rows(1.0));
    let node = container()
        .tone(tone)
        .class(DOT_GLYPH_ATOMS)
        .child(text("●"));
    (layout, node)
}

fn round_dot(tone: Tone, size: Size) -> (LayoutStyle, NodeBuilder) {
    let extent = match size {
        Size::Compact => 8.0,
        Size::Default => 12.0,
        Size::Wide | Size::Grow => 16.0,
    };
    #[allow(
        clippy::cast_possible_truncation,
        reason = "extent is 8.0..=16.0 → half_radius 4..=8, exact i16."
    )]
    let half_radius = Px((extent / 2.0) as i16);
    let inner_extent = (extent - 2.0_f32).max(2.0);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "inner_extent is 6.0..=14.0 → half 3..=7, exact i16."
    )]
    let inner_radius = Px((inner_extent / 2.0) as i16);
    let inner = container()
        .layout(dot_inner_layout(inner_extent))
        .style(crate::style::Style::new().radius(inner_radius.0))
        .tone(tone)
        .class(DOT_FILL_ATOMS);
    let node = container()
        .style(crate::style::Style::new().radius(half_radius.0))
        .tone(tone)
        .class(DOT_RING_ATOMS)
        .child(inner);
    (dot_outer_layout(extent), node)
}

static DOT_GLYPH_ATOMS: &[StyleAtom] = &[StyleAtom::FgToneStep(9)];

static DOT_FILL_ATOMS: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];

static DOT_RING_ATOMS: &[StyleAtom] = &[StyleAtom::BgToneStep(11)];

fn dot_outer_layout(extent: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(extent, extent)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn dot_inner_layout(extent: f32) -> LayoutStyle {
    LayoutStyle::flex_row().fixed_size(extent, extent)
}

impl StyleAccess for DotBuilder {
    fn style_mut(&mut self) -> &mut crate::style::Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for DotBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for DotBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[path = "dot_tests.rs"]
mod tests;
