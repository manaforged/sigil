#![allow(clippy::redundant_pub_crate)]

use sigil_kernel::Node;
use sigil_style::{FocusRing, Px, Rgba8, StyleEngine, Theme};

pub use sigil_style::unsupported_style_warning_count_for;
pub(crate) use sigil_style::{render_style_for, text_style_for};

use crate::corner_radii::CornerRadii;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedFocusRing {
    pub(crate) color: Rgba8,
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
        width: ring.stroke_width().max(0.0),
        offset: ring.offset_px().max(0.0),
        opacity: ring.opacity_value(),
    })
}

pub(crate) fn corner_radii_for(border_radius: Option<Px>) -> CornerRadii {
    border_radius.map_or(CornerRadii::ZERO, |radius| {
        CornerRadii::uniform(f32::from(radius.0))
    })
}
