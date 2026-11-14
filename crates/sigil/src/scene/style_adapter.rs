#![allow(clippy::redundant_pub_crate)]

use crate::kernel::Node;
use crate::style::{Color, FocusRing, StyleEngine, Theme};

pub(crate) use crate::style::{render_style_for, text_style_for};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedFocusRing {
    pub(crate) color: Color,
    pub(crate) width: f32,
    pub(crate) offset: f32,
    pub(crate) opacity: u16,
}

pub(crate) fn focus_ring_for(node: &Node, inherited: &Theme) -> Option<ResolvedFocusRing> {
    let ring = node.extension::<FocusRing>()?;
    if !ring.visible() {
        return None;
    }
    let theme = node.extension::<Theme>().unwrap_or(inherited);
    let engine = StyleEngine::new(theme);
    Some(ResolvedFocusRing {
        color: engine.resolve_color(ring.color_atom()),
        width: ring.stroke_width(theme.metrics).max(0.0),
        offset: ring.offset_px(theme.metrics).max(0.0),
        opacity: ring.opacity_value(),
    })
}
