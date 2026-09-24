#[cfg(feature = "scene")]
use std::rc::Rc;

use crate::kernel::TextContent;
use crate::kernel::{IntoNode, NodeBuilder, Role, container, text};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
#[cfg(feature = "scene")]
use crate::style::Token;
use crate::style::Tone;
use crate::style::{NodeStyleExt, Size, StyleAccess, StyleAtom};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

#[cfg(feature = "scene")]
mod svg;

#[cfg(feature = "scene")]
use svg::SvgGlyph;

static ICON_GLYPH_COLOR: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];

enum Glyph {
    Text(TextContent),
    #[cfg(feature = "scene")]
    Svg(Rc<str>),
}

pub struct IconBuilder {
    glyph: Glyph,
    tone: Tone,
    size: Size,
    label: Option<String>,
    style_accumulator: crate::style::Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn icon(glyph: impl Into<TextContent>) -> IconBuilder {
    IconBuilder::new(glyph)
}

#[cfg(feature = "scene")]
#[must_use]
pub fn icon_svg(svg: impl Into<Rc<str>>) -> IconBuilder {
    let mut builder = IconBuilder::new("");
    builder.glyph = Glyph::Svg(svg.into());
    builder
}

impl IconBuilder {
    #[must_use]
    pub fn new(glyph: impl Into<TextContent>) -> Self {
        Self {
            glyph: Glyph::Text(glyph.into()),
            tone: Tone::Neutral,
            size: Size::Default,
            label: None,
            style_accumulator: crate::style::Style::new(),
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
        let container_node = container()
            .role(role)
            .layout(self.layout_overrides.apply(icon_layout(extent)))
            .tone(self.tone);
        let mut container_node = match self.glyph {
            Glyph::Text(glyph) => container_node.child(
                text(glyph)
                    .class(glyph_class)
                    .class(ICON_GLYPH_COLOR)
                    .tone(self.tone)
                    .hittable(false),
            ),
            #[cfg(feature = "scene")]
            Glyph::Svg(svg) => container_node.child(SvgGlyph {
                svg,
                pixels: svg_pixels(extent * 0.8),
                token: Token::scale(self.tone.scale_role(), 11).unwrap_or(Token::Gray11),
                extent: extent * 0.8,
            }),
        };
        if let Some(label) = self.label {
            container_node = container_node.label(label);
        }
        if !self.style_accumulator.is_empty() {
            container_node = container_node.style(self.style_accumulator);
        }
        container_node
    }
}

#[cfg(feature = "scene")]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "icon extents are small positive constants"
)]
fn svg_pixels(extent: f32) -> u32 {
    (extent * 2.0).round() as u32
}

fn icon_layout(extent: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(extent, extent)
        .shrink(0.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

impl StyleAccess for IconBuilder {
    fn style_mut(&mut self) -> &mut crate::style::Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for IconBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for IconBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
