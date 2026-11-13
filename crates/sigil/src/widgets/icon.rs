use crate::kernel::{NodeBuilder, Role, TextContent, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
#[cfg(feature = "scene")]
use crate::style::Token;
use crate::style::{ControlSize, Style, StyleAtom, Tone};
use crate::widgets::Widget;

use super::layout_ext::LayoutOverrides;
use super::shared::caller_style;

pub(crate) const DISMISS_GLYPH: &str = "\u{d7}";
pub(crate) const CHECK_GLYPH: &str = "\u{2713}";

#[cfg(feature = "scene")]
mod svg;

static ICON_GLYPH_COLOR: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];

#[derive(Clone, Debug)]
enum Mark {
    Text(TextContent),
    #[cfg(feature = "scene")]
    Svg(std::sync::Arc<str>),
}

/// Builder of an icon.
#[derive(Debug)]
#[must_use]
pub struct IconBuilder {
    glyph: Mark,
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

/// What an icon or an icon button shows: a bundled `Icon` or a text glyph.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Glyph {
    /// A text glyph.
    Text(TextContent),
    /// A bundled icon.
    #[cfg(feature = "scene")]
    Icon(crate::icons::Icon),
    /// The source of an SVG image.
    #[cfg(feature = "scene")]
    Svg(std::sync::Arc<str>),
}

#[cfg(feature = "scene")]
impl Glyph {
    /// Makes a glyph from the source of an SVG image. A button made from it has no name: chain `.accessible(name)`.
    pub fn svg(source: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Svg(source.into())
    }
}

impl<T: Into<TextContent>> From<T> for Glyph {
    fn from(text: T) -> Self {
        Self::Text(text.into())
    }
}

#[cfg(feature = "scene")]
impl From<crate::icons::Icon> for Glyph {
    fn from(icon: crate::icons::Icon) -> Self {
        Self::Icon(icon)
    }
}

impl From<Glyph> for Mark {
    fn from(glyph: Glyph) -> Self {
        match glyph {
            Glyph::Text(text) => Self::Text(text),
            #[cfg(feature = "scene")]
            Glyph::Icon(icon) => Self::Svg(std::sync::Arc::from(icon.svg())),
            #[cfg(feature = "scene")]
            Glyph::Svg(source) => Self::Svg(source),
        }
    }
}

/// An icon: a bundled `Icon`, such as `icon(Icon::Folder)`, a text glyph, or `Glyph::svg(source)`.
pub fn icon(glyph: impl Into<Glyph>) -> IconBuilder {
    IconBuilder::new(glyph)
}

impl IconBuilder {
    /// Creates an icon in `Tone::Neutral` at `ControlSize::Default`.
    pub(crate) fn new(glyph: impl Into<Glyph>) -> Self {
        Self {
            glyph: Mark::from(glyph.into()),
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

    /// Sets the color tone; the default is `Tone::Neutral`.
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets the box to 16, 20, or 24 px for compact, default, or wide.
    pub const fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Sets the box to `px` px square, in place of the `size` step, and draws an SVG at that size.
    pub const fn px(mut self, px: f32) -> Self {
        self.px = Some(px);
        self
    }

    /// Draws an SVG icon in step 12 of its tone scale in place of step 11.
    pub const fn strong(mut self) -> Self {
        self.step = 12;
        self
    }

    #[cfg(feature = "scene")]
    pub(crate) const fn on_solid(mut self, yes: bool) -> Self {
        self.on_solid = yes;
        self
    }

    /// Sets the accessible name and gives the icon the image role.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn root(self) -> NodeBuilder {
        let (base, glyph_class) = match self.size {
            ControlSize::Compact => (16.0, "text-xs"),
            ControlSize::Default => (20.0, "text-base"),
            ControlSize::Wide => (24.0, "text-lg"),
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
            Mark::Text(glyph) => container_node.child(
                text(glyph)
                    .class(glyph_class)
                    .class(ICON_GLYPH_COLOR)
                    .tone_scope(self.tone)
                    .hittable(false),
            ),
            #[cfg(feature = "scene")]
            Mark::Svg(svg) => {
                let token = if self.on_solid {
                    Token::OnSolid
                } else {
                    Token::scale(self.tone, self.step).unwrap_or(Token::Gray11)
                };
                container_node.child(svg::glyph(svg, token, drawn))
            }
        };
        if let Some(label) = self.label {
            container_node = container_node.label(label);
        }
        caller_style(container_node, self.style_accumulator)
    }
}

builder_impls!([] IconBuilder: paint(style_accumulator), widget, node);

#[cfg(test)]
#[path = "icon_tests.rs"]
mod tests;
