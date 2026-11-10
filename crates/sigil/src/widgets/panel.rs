use super::layout_ext::LayoutOverrides;
use super::shared::caller_style;
use crate::kernel::{IntoNode, NodeBuilder, Role, container, text};
use crate::layout::LayoutStyle;
use crate::layout::Space;
use crate::style::{BORDER_WIDTH, Elevation, Layer, Radius, Style, StyleAtom, Token, space};
use crate::widgets::Layout;

const PANEL_MIN_HEIGHT: f32 = space(12);
const TITLE_GAP: f32 = space(2);

/// Builder of a panel.
pub struct PanelBuilder {
    /// The title, if any.
    pub title: Option<String>,
    /// The elevation.
    pub elevation: u32,
    content: Option<Box<dyn IntoNode>>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

impl core::fmt::Debug for PanelBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PanelBuilder")
            .field("title", &self.title)
            .field("elevation", &self.elevation)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A titled panel.
#[must_use]
pub fn panel() -> PanelBuilder {
    PanelBuilder::new()
}

impl PanelBuilder {
    /// Creates a panel builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            title: None,
            elevation: 1,
            content: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Sets the title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the elevation.
    #[must_use]
    pub fn elevation(mut self, tier: u32) -> Self {
        self.elevation = tier.min(3);
        self
    }

    /// Adds a child.
    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.content = Some(Box::new(child));
        self
    }

    fn root(self) -> NodeBuilder {
        let mut root = container()
            .role(Role::GenericContainer)
            .layout(
                LayoutStyle::flex_column()
                    .padding(Space::Lg)
                    .gap(Space::px(TITLE_GAP))
                    .fill_width()
                    .min_size(crate::layout::Length::Auto, PANEL_MIN_HEIGHT),
            )
            .class(Radius::Md.atom())
            .class(surface_atoms(self.elevation));

        if let Some(title) = self.title {
            root = root.child(
                text(title)
                    .class("text-sm font-medium")
                    .class(Token::OnSurface.text()),
            );
        }

        if let Some(child) = self.content {
            root = root.child(child);
        }

        caller_style(root, self.style_accumulator)
    }
}

impl Default for PanelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

builder_impls!([] PanelBuilder: paint(style_accumulator), widget, node);

const fn raised(shadow: Elevation) -> [StyleAtom; 4] {
    [
        Token::SurfaceHover.background(),
        StyleAtom::BorderWidth(BORDER_WIDTH),
        Token::Border.border(),
        StyleAtom::Shadow(shadow),
    ]
}

const fn surface_atoms(elevation: u32) -> &'static [StyleAtom] {
    match elevation {
        0 => Layer::Page.atoms(),
        1 => const { &raised(Elevation::Sm) },
        2 => const { &raised(Elevation::Base) },
        _ => const { &raised(Elevation::Md) },
    }
}
