#![allow(clippy::redundant_pub_crate)]

use crate::kernel::Node;
use crate::style::{Color, FocusRing, StyleEngine, Theme};

pub(crate) use crate::style::{render_style_for, text_style_for};

use crate::scene::corner_radii::CornerRadii;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedFocusRing {
    pub(crate) color: Color,
    pub(crate) width: f32,
    pub(crate) offset: f32,
    pub(crate) opacity: u16,
}

pub(crate) fn focus_ring_for(node: &Node, inherited: &Theme) -> Option<ResolvedFocusRing> {
    let ring = node.get_extension::<FocusRing>()?;
    if !ring.visible() {
        return None;
    }
    let theme = node.get_extension::<Theme>().unwrap_or(inherited);
    let engine = StyleEngine::new(theme);
    Some(ResolvedFocusRing {
        color: engine.resolve_color(ring.color_atom()),
        width: ring.stroke_width(theme.metrics).max(0.0),
        offset: ring.offset_px(theme.metrics).max(0.0),
        opacity: ring.opacity_value(),
    })
}

pub(crate) fn corner_radii_for(border_radius: Option<i16>) -> CornerRadii {
    border_radius.map_or(CornerRadii::ZERO, |radius| {
        CornerRadii::uniform(f32::from(radius))
    })
}
