use crate::kernel::{NodeBuilder, Role, TextContent, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
#[cfg(feature = "scene")]
use crate::style::Token;
use crate::style::{ControlSize, Style, StyleAtom, Tone};
use crate::widgets::Widget;

use super::layout_ext::LayoutOverrides;

#[cfg(feature = "scene")]
mod svg;

static ICON_GLYPH_COLOR: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];

#[derive(Clone)]
enum Glyph {
    Text(TextContent),
    #[cfg(feature = "scene")]
    Svg(std::sync::Arc<str>),
}

/// Builder of an icon.
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

/// What `icon` accepts: a bundled `Icon` or any text glyph.
#[derive(Clone)]
pub struct IconSource(Glyph);

impl IntoIconSource for IconSource {
    fn into_icon_source(self) -> IconSource {
        self
    }
}

/// A value that can be drawn as an icon.
pub trait IntoIconSource {
    /// Converts this into icon source to icon source.
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
        IconSource(Glyph::Svg(std::sync::Arc::from(self.svg())))
    }
}

/// An icon: a bundled `Icon`, such as `icon(Icon::Folder)`, or a text glyph.
#[must_use]
pub fn icon(glyph: impl IntoIconSource) -> IconBuilder {
    IconBuilder::new(glyph)
}

/// An SVG icon.
#[cfg(feature = "scene")]
#[must_use]
pub fn icon_svg(svg: impl Into<std::sync::Arc<str>>) -> IconBuilder {
    let mut builder = IconBuilder::new("");
    builder.glyph = Glyph::Svg(svg.into());
    builder
}

impl IconBuilder {
    /// Creates an icon builder.
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

    /// Sets the tone.
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets the size.
    #[must_use]
    pub const fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Sets the px.
    #[must_use]
    pub const fn px(mut self, px: f32) -> Self {
        self.px = Some(px);
        self
    }

    /// Applies the strong option.
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

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn root(self) -> NodeBuilder {
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
        let container_node = container().role(role).with(layout).tone_scope(self.tone);
        let mut container_node = match self.glyph {
            Glyph::Text(glyph) => container_node.child(
                text(glyph)
                    .class(glyph_class)
                    .class(ICON_GLYPH_COLOR)
                    .tone_scope(self.tone)
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

builder_impls!([] IconBuilder: paint(style_accumulator), widget, node);

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
