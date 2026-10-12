#[cfg(feature = "scene")]
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
#[cfg(feature = "scene")]
use crate::style::Token;
use crate::style::{ControlSize, NodeStyleExt, PaintTarget, Style, StyleAtom, StyleSlot, Tone};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

#[cfg(feature = "scene")]
mod svg;

static ICON_GLYPH_COLOR: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];

#[derive(Clone)]
enum Glyph {
    Text(TextContent),
    #[cfg(feature = "scene")]
    Svg(Rc<str>),
}

pub struct IconBuilder {
    glyph: Glyph,
    tone: Tone,
    size: ControlSize,
    px: Option<f32>,
    step: u8,
    #[cfg(feature = "scene")]
    on_solid: bool,
    label: Option<String>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[derive(Clone)]
pub struct IconSource(Glyph);

impl IntoIconSource for IconSource {
    fn into_icon_source(self) -> IconSource {
        self
    }
}

pub trait IntoIconSource {
    fn into_icon_source(self) -> IconSource;
}

impl<T: Into<TextContent>> IntoIconSource for T {
    fn into_icon_source(self) -> IconSource {
        IconSource(Glyph::Text(self.into()))
    }
}

#[cfg(feature = "scene")]
impl IntoIconSource for crate::icons::Icon {
    fn into_icon_source(self) -> IconSource {
        IconSource(Glyph::Svg(Rc::from(self.svg())))
    }
}

#[must_use]
pub fn icon(glyph: impl IntoIconSource) -> IconBuilder {
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
    pub fn new(glyph: impl IntoIconSource) -> Self {
        Self {
            glyph: glyph.into_icon_source().0,
            tone: Tone::Neutral,
            size: ControlSize::Default,
            px: None,
            step: 11,
            #[cfg(feature = "scene")]
            on_solid: false,
            label: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub const fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub const fn px(mut self, px: f32) -> Self {
        self.px = Some(px);
        self
    }

    #[must_use]
    pub const fn strong(mut self) -> Self {
        self.step = 12;
        self
    }

    #[cfg(feature = "scene")]
    #[must_use]
    pub(crate) const fn on_solid(mut self, yes: bool) -> Self {
        self.on_solid = yes;
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let (base, glyph_class) = match self.size {
            ControlSize::Compact => (16.0, "text-xs"),
            ControlSize::Default => (20.0, "text-base"),
            ControlSize::Wide | ControlSize::Grow => (24.0, "text-lg"),
        };
        let extent = self.px.unwrap_or(base);
        #[cfg(feature = "scene")]
        let drawn = self.px.unwrap_or(extent * 0.8);
        let role = if self.label.is_some() {
            Role::Image
        } else {
            Role::GenericContainer
        };
        let layout = LayoutStyle::flex_row()
            .fixed_size(extent, extent)
            .shrink(0.0)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center);
        let container_node = container()
            .role(role)
            .with(self.layout_overrides.apply(layout))
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
            Glyph::Svg(svg) => container_node.child(svg::SvgGlyph {
                svg,
                pixels: svg_pixels(drawn),
                token: if self.on_solid {
                    Token::OnSolid
                } else {
                    Token::scale(self.tone.scale_role(), self.step).unwrap_or(Token::Gray11)
                },
                extent: drawn,
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

impl PaintTarget for IconBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for IconBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for IconBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
