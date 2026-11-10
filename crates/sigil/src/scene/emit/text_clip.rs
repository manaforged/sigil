use std::hash::{Hash as _, Hasher as _};
use std::rc::Rc;

use ahash::AHasher;

use crate::kernel::{Node, NodeType};
use crate::layout::LayoutResult;
use crate::scene::command::Gradient;
use crate::style::{RenderStyle, Theme};

pub(super) struct TextFill {
    pub(super) gradient: Gradient,
    pub(super) key: u64,
}

impl TextFill {
    pub(super) fn of(node: &Node, style: &RenderStyle, theme: &Theme) -> Option<Rc<Self>> {
        if !style.paint.clip_text || !matches!(node.kind, NodeType::Container) {
            return None;
        }
        let rect = node.get_extension::<LayoutResult>()?.rect();
        let gradient = super::background::fill(rect, style, theme)?;
        let mut hasher = AHasher::default();
        format!("{gradient:?}").hash(&mut hasher);
        Some(Rc::new(Self {
            gradient,
            key: hasher.finish().max(1),
        }))
    }
}
