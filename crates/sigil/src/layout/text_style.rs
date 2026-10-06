#![allow(clippy::redundant_pub_crate)]

use std::sync::OnceLock;

use crate::kernel::{Cx, Node, NodeId};
use crate::style::locale::Direction;
use crate::style::{StyleBinding, StyleEngine, TextStyle, Theme};
use smallvec::SmallVec;

pub(crate) fn apply_text_binding(
    engine: StyleEngine<'_>,
    binding: &StyleBinding,
    style: &mut TextStyle,
) {
    for atom in binding.cascade() {
        engine.apply_text(&[atom], style);
    }
    if let Some(overlay) = binding.animator_overlay() {
        StyleEngine::overlay_text(overlay, style);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct InheritedText<'a> {
    pub(crate) style: &'a TextStyle,
    pub(crate) direction: Direction,
    pub(crate) sig: u64,
}

impl<'a> InheritedText<'a> {
    pub(crate) fn new(style: &'a TextStyle, direction: Direction) -> Self {
        Self {
            style,
            direction,
            sig: inherited_signature(style, direction),
        }
    }

    pub(crate) fn under(self, node: &Node) -> Self {
        let direction = node_direction(node, self.direction);
        if direction == self.direction {
            self
        } else {
            Self::new(self.style, direction)
        }
    }

    pub(crate) fn layered(self, layered: Option<&'a TextStyle>) -> Self {
        layered.map_or(self, |style| Self::new(style, self.direction))
    }
}

const RTL_SIGNATURE_BIT: u64 = 1 << 63;

pub(crate) fn inherited_signature(style: &TextStyle, direction: Direction) -> u64 {
    let base = text_style_signature(style) & !RTL_SIGNATURE_BIT;
    if direction.is_rtl() {
        base | RTL_SIGNATURE_BIT
    } else {
        base
    }
}

pub(crate) const fn signature_is_rtl(sig: u64) -> bool {
    sig & RTL_SIGNATURE_BIT != 0
}

fn explicit_direction(node: &Node) -> Option<Direction> {
    node.get_extension::<Direction>()
        .copied()
        .or_else(|| node.get_extension::<Theme>().map(|theme| theme.direction))
}

pub(crate) fn node_direction(node: &Node, inherited: Direction) -> Direction {
    explicit_direction(node).unwrap_or(inherited)
}

pub(crate) fn inherited_direction(cx: &Cx, id: NodeId) -> Direction {
    let mut current = cx.parent(id);
    while let Some(ancestor) = current {
        if let Some(direction) = cx.get(ancestor).and_then(explicit_direction) {
            return direction;
        }
        current = cx.parent(ancestor);
    }
    Direction::Ltr
}

pub(crate) fn text_style_signature(style: &TextStyle) -> u64 {
    crate::style::text_identity::DiscreteTextStyle::identity(style)
}

pub(crate) fn inherited_text_style(cx: &Cx, id: NodeId) -> TextStyle {
    let mut chain = SmallVec::<[NodeId; 16]>::new();
    let mut current = cx.parent(id);
    while let Some(ancestor) = current {
        chain.push(ancestor);
        current = cx.parent(ancestor);
    }
    let mut style = TextStyle::default();
    for ancestor in chain.iter().rev() {
        let Some(node) = cx.get(*ancestor) else {
            continue;
        };
        if let Some(binding) = node.get_extension::<StyleBinding>()
            && let Some(layered) = TextStyle::layered_over_binding(&style, binding)
        {
            style = layered;
        }
    }
    style
}

pub(crate) fn theme_for(node: &Node) -> &Theme {
    node.get_extension::<Theme>().unwrap_or_else(default_theme)
}

fn default_theme<'theme>() -> &'theme Theme {
    static DEFAULT_THEME: OnceLock<Theme> = OnceLock::new();
    DEFAULT_THEME.get_or_init(Theme::default)
}
