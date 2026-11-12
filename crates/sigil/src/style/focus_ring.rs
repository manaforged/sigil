use crate::reactive::Signal;

use crate::style::{ColorAtom, Metrics, Token};

/// The focus ring of a node: when it shows, and its color, width, offset and opacity.
#[derive(Clone, Copy, Debug, Default)]
pub struct FocusRing {
    visible: Option<Signal<bool>>,
    color: Option<ColorAtom>,
    width: Option<f32>,
    offset: Option<f32>,
    opacity: Option<u16>,
}

impl From<Signal<bool>> for FocusRing {
    fn from(visible: Signal<bool>) -> Self {
        Self {
            visible: Some(visible),
            ..Self::default()
        }
    }
}

impl FocusRing {
    /// Sets the color of the focus ring.
    #[must_use]
    pub fn color(mut self, color: impl Into<ColorAtom>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the stroke width of the focus ring in pixels.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets the distance between the node and the focus ring in pixels.
    #[must_use]
    pub const fn offset(mut self, offset: f32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the opacity in ten-thousandths.
    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        debug_assert!(opacity <= 10_000);
        self.opacity = Some(opacity);
        self
    }

    pub(crate) fn merge(&mut self, over: Self) {
        self.visible = over.visible.or(self.visible);
        self.color = over.color.or(self.color);
        self.width = over.width.or(self.width);
        self.offset = over.offset.or(self.offset);
        self.opacity = over.opacity.or(self.opacity);
    }

    kittest_pub! {
        /// Returns whether the ring is shown now.
        #[must_use]
        pub fn visible(&self) -> bool {
            self.visible.is_some_and(|visible| visible.get())
        }
    }

    pub(crate) fn color_atom(&self) -> ColorAtom {
        self.color.unwrap_or_else(|| Token::Ring.color())
    }

    pub(crate) fn stroke_width(&self, metrics: Metrics) -> f32 {
        self.width.unwrap_or_else(|| metrics.ring_width())
    }

    pub(crate) fn offset_px(&self, metrics: Metrics) -> f32 {
        self.offset.unwrap_or_else(|| metrics.ring_offset())
    }

    pub(crate) fn opacity_value(&self) -> u16 {
        self.opacity.unwrap_or(10_000)
    }
}
