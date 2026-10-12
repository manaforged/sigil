use crate::reactive::Signal;

use crate::style::{ColorAtom, Metrics, Rgba8, Token};

#[derive(Clone, Debug)]
pub struct FocusRing {
    visible: FocusRingVisibility,
    color: ColorAtom,
    width: Option<f32>,
    offset: Option<f32>,
    opacity: u16,
}

impl FocusRing {
    #[must_use]
    pub const fn new(visible: Signal<bool>) -> Self {
        Self::with_visibility(FocusRingVisibility::Reactive(visible))
    }

    #[must_use]
    pub const fn static_visible(visible: bool) -> Self {
        Self::with_visibility(FocusRingVisibility::Static(visible))
    }

    const fn with_visibility(visible: FocusRingVisibility) -> Self {
        Self {
            visible,
            color: Token::Ring.color(),
            width: None,
            offset: None,
            opacity: 10_000,
        }
    }

    #[must_use]
    pub const fn color(mut self, color: Rgba8) -> Self {
        self.color = ColorAtom::Rgba8(color);
        self
    }

    #[must_use]
    pub const fn theme_color(mut self, token: Token) -> Self {
        self.color = token.color();
        self
    }

    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    #[must_use]
    pub const fn offset(mut self, offset: f32) -> Self {
        self.offset = Some(offset);
        self
    }

    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        debug_assert!(opacity <= 10_000);
        self.opacity = opacity;
        self
    }

    #[must_use]
    pub fn visible(&self) -> bool {
        match self.visible {
            FocusRingVisibility::Static(visible) => visible,
            FocusRingVisibility::Reactive(visible) => visible.get(),
        }
    }

    #[must_use]
    pub const fn color_atom(&self) -> ColorAtom {
        self.color
    }

    #[must_use]
    pub fn stroke_width(&self, metrics: Metrics) -> f32 {
        self.width.unwrap_or_else(|| metrics.ring_width())
    }

    #[must_use]
    pub fn offset_px(&self, metrics: Metrics) -> f32 {
        self.offset.unwrap_or_else(|| metrics.ring_offset())
    }

    #[must_use]
    pub const fn opacity_value(&self) -> u16 {
        self.opacity
    }
}

#[derive(Clone, Copy, Debug)]
enum FocusRingVisibility {
    Static(bool),
    Reactive(Signal<bool>),
}
