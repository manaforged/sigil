#![allow(clippy::redundant_pub_crate)]

use std::hash::{Hash as _, Hasher as _};
use std::sync::OnceLock;

use ahash::AHasher;
use sigil_kernel::{Cx, Node, NodeId};
use sigil_style::{StyleBinding, StyleEngine, TextLineHeight, TextStyle, Theme};
use smallvec::SmallVec;

pub(crate) fn apply_text_binding(
    engine: StyleEngine<'_>,
    binding: &StyleBinding,
    style: &mut TextStyle,
) {
    engine.apply_text(binding.static_slice(), style);
    engine.apply_text(binding.dynamic_slice(), style);
    for reactive in binding.reactive_channels() {
        engine.apply_text(reactive, style);
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
    let mut hasher = AHasher::default();
    style.font_family.hash(&mut hasher);
    style.font_size.to_bits().hash(&mut hasher);
    style.font_weight.hash(&mut hasher);
    match style.line_height {
        TextLineHeight::FontSizeRelative(value) => {
            0_u8.hash(&mut hasher);
            value.to_bits().hash(&mut hasher);
        }
        TextLineHeight::Absolute(value) => {
            1_u8.hash(&mut hasher);
            value.to_bits().hash(&mut hasher);
        }
    }
    hasher.finish()
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
