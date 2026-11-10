use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::LayoutStyle;
use crate::reactive::{Reactive, Signal};
use crate::style::{BORDER_WIDTH, Elevation, Layer, Radius, StyleAtom, Token};
use crate::widgets::Widget;

use super::super::shared::Activate;
use super::ROW_SELECTED;
use super::activatable::activatable;

static CARD: &[StyleAtom] = Layer::Raised.atoms();

static CARD_SELECTED: &[StyleAtom] = &[
    Radius::Md.atom(),
    ROW_SELECTED.background(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Accent8.border(),
    StyleAtom::Shadow(Elevation::Sm),
];

/// Builds a card.
#[must_use]
pub struct CardBuilder {
    node: NodeBuilder,
    selected: Option<Signal<bool>>,
    activate: Option<Activate>,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for CardBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CardBuilder")
            .field("node", &self.node)
            .field("selected", &self.selected)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A bordered card.
pub fn card(child: impl IntoNode + 'static) -> CardBuilder {
    CardBuilder {
        node: container()
            .role(Role::GenericContainer)
            .layout(
                LayoutStyle::flex_column()
                    .padding(Space::Md)
                    .gap(Space::Sm)
                    .fill_width()
                    .min_width(0.0),
            )
            .child(child),
        selected: None,
        activate: None,
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl CardBuilder {
    /// Sets the selected.
    pub fn selected(mut self, selected: impl Reactive<bool>) -> Self {
        self.selected = Some(selected.into_signal());
        self
    }

    /// Sets the activate.
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    #[must_use]
    fn root(self) -> NodeBuilder {
        let node = match self.selected {
            Some(selected) => self
                .node
                .selected(selected)
                .style_reactive(move || if selected.get() { CARD_SELECTED } else { CARD }),
            None => self.node.class(CARD),
        };
        match self.activate {
            Some(activate) => activatable(node, Role::Button, &activate),
            None => node,
        }
    }
}

impl From<CardBuilder> for NodeBuilder {
    fn from(card: CardBuilder) -> Self {
        card.into_node_builder()
    }
}

builder_impls!([] CardBuilder: widget, node(pub));
