use super::layout_ext::LayoutOverrides;
use crate::kernel::{IntoNode, NodeBuilder, Role, container, text};
use crate::layout::LayoutStyle;
use crate::style::{BORDER_WIDTH, Radius, Style, StyleAtom, Token, space};
use crate::widgets::Widget;

const PANEL_MIN_HEIGHT: f32 = space(12);
const DEFAULT_PADDING: f32 = space(4);
const TITLE_GAP: f32 = space(2);

/// Builder of a panel.
pub struct PanelBuilder {
    /// The title, if any.
    pub title: Option<String>,
    /// The padding.
    pub padding: f32,
    /// The elevation.
    pub elevation: u32,
    content: Option<Box<dyn IntoNode>>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
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
            padding: DEFAULT_PADDING,
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

    /// Sets the padding.
    #[must_use]
    pub const fn padding(mut self, px: f32) -> Self {
        self.padding = px;
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
                    .padding(self.padding)
                    .gap(TITLE_GAP)
                    .fill_width()
                    .min_size(crate::layout::Dimension::Auto, PANEL_MIN_HEIGHT),
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

        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }

        root
    }
}

impl Default for PanelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

builder_impls!([] PanelBuilder: paint(style_accumulator), widget, node);

const fn surface_atoms(elevation: u32) -> &'static [StyleAtom] {
    match elevation {
        0 => const { &[Token::Surface.bg()] },
        _ => {
            const {
                &[
                    Token::SurfaceHover.bg(),
                    StyleAtom::BorderWidth(BORDER_WIDTH),
                    Token::Border.border(),
                ]
            }
        }
    }
}
