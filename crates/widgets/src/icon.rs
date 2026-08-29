use crate::widget_dev::*;
use sigil_kernel::TextContent;
use sigil_style::Tone;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

static ICON_GLYPH_COLOR: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];

pub struct IconBuilder {
    glyph: TextContent,
    tone: Tone,
    size: Size,
    label: Option<String>,
    style_accumulator: sigil_style::Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn icon(glyph: impl Into<TextContent>) -> IconBuilder {
    IconBuilder::new(glyph)
}

impl IconBuilder {
    #[must_use]
    pub fn new(glyph: impl Into<TextContent>) -> Self {
        Self {
            glyph: glyph.into(),
            tone: Tone::Neutral,
            size: Size::Default,
            label: None,
            style_accumulator: sigil_style::Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let extent = match self.size {
            Size::Compact => 16.0,
            Size::Default => 20.0,
            Size::Wide | Size::Grow => 24.0,
        };
        let glyph_class = match self.size {
            Size::Compact => "text-xs",
            Size::Default => "text-base",
            Size::Wide | Size::Grow => "text-lg",
        };
        let role = self
            .label
            .as_ref()
            .map_or(Role::GenericContainer, |_| Role::Image);
        let mut container_node = container()
            .role(role)
            .layout(self.layout_overrides.apply(icon_layout(extent)))
            .tone(self.tone)
            .child(
                text(self.glyph)
                    .class(glyph_class)
                    .class(ICON_GLYPH_COLOR)
                    .tone(self.tone)
                    .hittable(false),
            );
        if let Some(label) = self.label {
            container_node = container_node.label(label);
        }
        if !self.style_accumulator.is_empty() {
            container_node = container_node.style(self.style_accumulator);
        }
        container_node
    }
}

fn icon_layout(extent: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(extent, extent)
        .shrink(0.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

impl StyleAccess for IconBuilder {
    fn style_mut(&mut self) -> &mut sigil_style::Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for IconBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for IconBuilder {
    fn build(self, cx: &mut sigil_kernel::Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
