use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, container};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Reactive, Signal};
use crate::style::{BORDER_WIDTH, Radius, StyleAtom, Token, space};

use super::super::shared::Activate;
use super::ROW_SELECTED;
use super::activatable::activatable;

static CARD: &[StyleAtom] = &[
    Radius::Md.atom(),
    Token::Surface.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Border.border(),
];

static CARD_SELECTED: &[StyleAtom] = &[
    Radius::Md.atom(),
    ROW_SELECTED.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Accent8.border(),
];

#[must_use]
pub struct CardBuilder {
    node: NodeBuilder,
    selected: Option<Signal<bool>>,
    activate: Option<Activate>,
}

pub fn card(child: impl IntoNode + 'static) -> CardBuilder {
    CardBuilder {
        node: container()
            .role(Role::GenericContainer)
            .layout(
                LayoutStyle::flex_column()
                    .padding(space(3))
                    .gap(space(2))
                    .fill_width()
                    .min_width(0.0),
            )
            .child(child),
        selected: None,
        activate: None,
    }
}

impl CardBuilder {
    pub fn selected(mut self, selected: impl Reactive<bool>) -> Self {
        self.selected = Some(selected.into_signal());
        self
    }

    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
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

impl IntoNode for CardBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}
