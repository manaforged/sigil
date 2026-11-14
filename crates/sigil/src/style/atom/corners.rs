use super::PaintStyle;
use crate::scene::corner_radii::CornerRadii;
use crate::style::{AnimatorOverlay, NumericAtomKind};

pub(crate) const ALL: u8 = 0b1111;
pub(crate) const TOP: u8 = 0b0011;
pub(crate) const RIGHT: u8 = 0b0110;
pub(crate) const BOTTOM: u8 = 0b1100;
pub(crate) const LEFT: u8 = 0b1001;

pub(crate) const KINDS: [NumericAtomKind; 4] = [
    NumericAtomKind::RadiusTopLeft,
    NumericAtomKind::RadiusTopRight,
    NumericAtomKind::RadiusBottomRight,
    NumericAtomKind::RadiusBottomLeft,
];

pub(crate) fn each(mask: u8) -> impl Iterator<Item = usize> {
    (0..KINDS.len()).filter(move |index| mask & (1 << index) != 0)
}

impl PaintStyle {
    pub(crate) fn round(&mut self, mask: u8, px: i16) {
        for index in each(mask) {
            if let Some(corner) = self.corners.get_mut(index) {
                *corner = Some(px);
            }
        }
    }

    pub(crate) fn radii(&self) -> CornerRadii {
        CornerRadii(self.corners.map(|corner| corner.map_or(0.0, f32::from)))
    }

    pub(crate) fn corner(&self, kind: NumericAtomKind) -> Option<f32> {
        let index = KINDS.iter().position(|owner| *owner == kind)?;
        let px = self.corners.get(index).copied().flatten();
        Some(px.map_or(0.0, f32::from))
    }

    pub(crate) fn overlay_corners(&mut self, overlay: &dyn AnimatorOverlay) {
        for (corner, kind) in self.corners.iter_mut().zip(KINDS) {
            if let Some(value) = overlay.value_for(kind) {
                *corner = Some(crate::style::engine::encode_px(value).0);
            }
        }
    }
}
