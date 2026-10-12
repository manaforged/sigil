use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container, text};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::style::{BORDER_WIDTH, PaintTarget, Radius, Style, StyleAtom, StyleSlot, Token, space};

const PANEL_MIN_HEIGHT: f32 = space(12);
const DEFAULT_PADDING: f32 = space(4);
const TITLE_GAP: f32 = space(2);

pub struct PanelBuilder {
    pub title: Option<String>,
    pub padding: f32,
    pub elevation: u32,
    content: Option<Box<dyn IntoNode>>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn panel() -> PanelBuilder {
    PanelBuilder::new()
}

impl PanelBuilder {
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

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub const fn padding(mut self, px: f32) -> Self {
        self.padding = px;
        self
    }

    #[must_use]
    pub fn elevation(mut self, tier: u32) -> Self {
        self.elevation = tier.min(3);
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.content = Some(Box::new(child));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let mut root = container()
            .role(Role::GenericContainer)
            .layout(
                self.layout_overrides.apply(
                    LayoutStyle::flex_column()
                        .padding(self.padding)
                        .gap(TITLE_GAP)
                        .fill_width()
                        .min_size(crate::layout::Dimension::Auto, PANEL_MIN_HEIGHT),
                ),
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

impl PaintTarget for PanelBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for PanelBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for PanelBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

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
