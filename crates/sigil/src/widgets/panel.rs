//! Panel widget — themed surface card with optional title.

use kernel::{Cx, IntoNode, NodeBuilder, Role, container, text};
use layout::prelude::{LayoutBuilderExt, LayoutStyle};
use style::{NodeStyleExt, Style, StyleAccess, StyleAtom};

const PANEL_WIDTH: f32 = 320.0;
const PANEL_MIN_HEIGHT: f32 = 48.0;
const DEFAULT_PADDING: f32 = 16.0;
const TITLE_HEIGHT: f32 = 40.0;

/// Builder returned by [`panel`].
pub struct PanelBuilder {
    /// Optional title rendered in the panel header.
    pub title: Option<String>,
    /// Inner padding in logical pixels.
    pub padding: f32,
    /// Shadow/elevation tier: 0 = none, 1 = subtle, 2 = card, 3 = modal.
    pub elevation: u32,
    content: Option<Box<dyn IntoNode>>,
    style_accumulator: Style,
}

/// Create a panel widget — a themed surface card.
#[must_use]
pub fn panel() -> PanelBuilder {
    PanelBuilder::new()
}

impl PanelBuilder {
    /// Create a panel builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            title: None,
            padding: DEFAULT_PADDING,
            elevation: 1,
            content: None,
            style_accumulator: Style::new(),
        }
    }

    /// Set the panel title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set inner padding in logical pixels.
    #[must_use]
    pub const fn padding(mut self, px: f32) -> Self {
        self.padding = px;
        self
    }

    /// Set elevation tier (0 = none, 1 = subtle, 2 = card, 3 = modal).
    #[must_use]
    pub fn elevation(mut self, tier: u32) -> Self {
        self.elevation = tier.min(3);
        self
    }

    /// Attach a child node (panel body content).
    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.content = Some(Box::new(child));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let mut root = container()
            .role(Role::GenericContainer)
            .layout(panel_layout(self.padding))
            .class("rounded-md")
            .class(surface_atoms(self.elevation));

        if let Some(title) = self.title {
            root = root.child(panel_title(title));
        }

        if let Some(child) = self.content {
            root = root.child(child);
        }

        // Styled accumulator installs LAST so user-supplied tokens
        // layer on top of the panel's elevation/rounding defaults
        // (additive composition per DR-049 §10).
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

// DS2.5 — `impl StateHost` removed. Panel is a static surface widget
// that doesn't track interaction state; its previous `StateHost` impl
// silently dropped `.hover() / .focus() / .pressed() / .disabled()`
// atoms because `into_node_builder` never called `take_state_overlay`.
// Reintroduce only if panel grows interaction-state tracking AND the
// `into_node_builder` pattern in button.rs is replicated here.

impl IntoNode for PanelBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn panel_layout(padding: f32) -> LayoutStyle {
    LayoutStyle::flex_column()
        .padding(padding)
        .min_size(PANEL_WIDTH, PANEL_MIN_HEIGHT)
}

fn panel_title(title: String) -> impl IntoNode {
    text(title)
        .layout(LayoutStyle::block().height(TITLE_HEIGHT))
        .class("text-on-surface")
}

const fn surface_atoms(elevation: u32) -> &'static [StyleAtom] {
    match elevation {
        0 => &[StyleAtom::BackgroundThemeRef("surface")],
        _ => &[StyleAtom::BackgroundThemeRef("surface-hover")],
    }
}
