use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container, text};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::style::{
    BORDER_WIDTH, NodeStyleExt, Radius, Style, StyleAccess, StyleAtom, Token, space,
};

const PANEL_MIN_HEIGHT: f32 = space(12);
const DEFAULT_PADDING: f32 = space(4);
const TITLE_GAP: f32 = space(2);

pub struct PanelBuilder {
    pub title: Option<String>,
    pub padding: f32,
    pub elevation: u32,
    content: Option<Box<dyn IntoNode>>,
    style_accumulator: Style,
    width: Option<f32>,
}

impl PanelBuilder {
    #[must_use]
    pub const fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }
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
            width: None,
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
            .layout(panel_layout(self.padding, self.width))
            .class(Radius::Md.atom())
            .class(surface_atoms(self.elevation));

        if let Some(title) = self.title {
            root = root.child(panel_title(title));
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

impl StyleAccess for PanelBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for PanelBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn panel_layout(padding: f32, width: Option<f32>) -> LayoutStyle {
    let base = LayoutStyle::flex_column().padding(padding).gap(TITLE_GAP);
    let base = match width {
        Some(px) => base.width(px),
        None => base.fill_width(),
    };
    base.min_size(crate::layout::Dimension::Auto, PANEL_MIN_HEIGHT)
}

fn panel_title(title: String) -> impl IntoNode {
    text(title)
        .class("text-sm font-medium")
        .class(Token::OnSurface.text())
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
