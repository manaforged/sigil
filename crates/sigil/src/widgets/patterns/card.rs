use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, container, text};
use crate::layout::LayoutStyle;
use crate::reactive::{Reactive, Signal};
use crate::style::{BORDER_WIDTH, Elevation, Layer, Radius, Style, StyleAtom, Token, Tone};
use crate::widgets::Widget;

use super::super::shared::{Activate, caller_style};
use super::ROW_SELECTED;

static CARD_SELECTED: &[StyleAtom] = &[
    Radius::Md.atom(),
    ROW_SELECTED.background(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Tone::Primary.step(8).border(),
    StyleAtom::Shadow(Elevation::Sm),
];

/// Builds a card.
#[must_use]
pub struct CardBuilder {
    node: NodeBuilder,
    title: Option<String>,
    layer: Layer,
    content: Box<dyn IntoNode>,
    selected: Option<Signal<bool>>,
    activate: Option<Activate>,
    style_accumulator: Style,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for CardBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CardBuilder")
            .field("node", &self.node)
            .field("title", &self.title)
            .field("selected", &self.selected)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A bordered card. Chain `.title(..)` for a heading and `.layer(Layer)` for another depth.
pub fn card(child: impl IntoNode + 'static) -> CardBuilder {
    CardBuilder {
        node: container().role(Role::GenericContainer).layout(
            LayoutStyle::flex_column()
                .padding(Space::Md)
                .gap(Space::Sm)
                .fill_width()
                .min_width(0.0),
        ),
        title: None,
        layer: Layer::Raised,
        content: Box::new(child),
        selected: None,
        activate: None,
        style_accumulator: Style::new(),
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl CardBuilder {
    /// Sets the depth of the card's surface.
    pub const fn layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    /// Sets the title shown above the content.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Draws the card with the selected fill and an accent border while `selected` is true.
    pub fn selected(mut self, selected: impl Reactive<bool>) -> Self {
        self.selected = Some(selected.into_signal());
        self
    }

    /// Makes the card a button that runs `activate` when the user activates it.
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    fn root(self) -> NodeBuilder {
        let titled = match self.title {
            Some(title) => self.node.child(
                text(title)
                    .class("text-sm font-medium")
                    .class(Token::OnSurface.text()),
            ),
            None => self.node,
        }
        .child(self.content);
        let rest = self.layer.atoms();
        let node = match self.selected {
            Some(selected) => titled
                .selected(selected)
                .style_reactive(move || if selected.get() { CARD_SELECTED } else { rest }),
            None => titled.class(rest),
        };
        let node = match self.activate {
            Some(activate) => node.role(Role::Button).activate_with(activate),
            None => node,
        };
        caller_style(node, self.style_accumulator)
    }
}

impl From<CardBuilder> for NodeBuilder {
    fn from(card: CardBuilder) -> Self {
        card.into_node_builder()
    }
}

builder_impls!([] CardBuilder: paint(style_accumulator), states(style_accumulator), widget, node(pub));
