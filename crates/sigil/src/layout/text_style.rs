#![allow(clippy::redundant_pub_crate)]

use std::sync::OnceLock;

use crate::kernel::{Cx, Node, NodeId};
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
    pub(crate) sig: u64,
}

impl<'a> InheritedText<'a> {
    pub(crate) fn new(style: &'a TextStyle) -> Self {
        Self {
            style,
            sig: text_style_signature(style),
        }
    }
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
