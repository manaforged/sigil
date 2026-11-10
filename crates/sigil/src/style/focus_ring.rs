use crate::reactive::Signal;

use crate::style::{ColorAtom, Metrics, Token};

/// Focus ring settings on a node.
#[derive(Clone, Debug)]
pub struct FocusRing {
    visible: Option<Signal<bool>>,
    pub(crate) color: ColorAtom,
    pub(crate) width: Option<f32>,
    pub(crate) offset: Option<f32>,
    pub(crate) opacity: u16,
}

impl Default for FocusRing {
    fn default() -> Self {
        Self {
            visible: None,
            color: Token::Ring.color(),
            width: None,
            offset: None,
            opacity: 10_000,
        }
    }
}

impl FocusRing {
    pub(crate) const fn show_when(&mut self, visible: Signal<bool>) {
        self.visible = Some(visible);
    }

    /// Returns whether the ring is shown now.
    #[must_use]
    pub fn visible(&self) -> bool {
        self.visible.is_some_and(|visible| visible.get())
    }

    pub(crate) const fn color_atom(&self) -> ColorAtom {
        self.color
    }

    pub(crate) fn stroke_width(&self, metrics: Metrics) -> f32 {
        self.width.unwrap_or_else(|| metrics.ring_width())
    }

    pub(crate) fn offset_px(&self, metrics: Metrics) -> f32 {
        self.offset.unwrap_or_else(|| metrics.ring_offset())
    }

    pub(crate) const fn opacity_value(&self) -> u16 {
        self.opacity
    }
}
