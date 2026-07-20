//! Focus-ring style extension.

use sigil_reactive::Signal;

use crate::{ColorAtom, Rgba8};

/// Visual focus-ring contract attached to focusable widgets.
#[derive(Clone, Debug)]
pub struct FocusRing {
    visible: FocusRingVisibility,
    color: ColorAtom,
    width: f32,
    offset: f32,
    opacity: u16,
}

impl FocusRing {
    /// Create a theme-accent focus ring driven by a visibility signal.
    #[must_use]
    pub const fn new(visible: Signal<bool>) -> Self {
        Self {
            visible: FocusRingVisibility::Reactive(visible),
            color: ColorAtom::ThemeRef("accent"),
            width: 2.0,
            offset: 2.0,
            opacity: 10_000,
        }
    }

    /// Create a focus ring with fixed visibility.
    #[must_use]
    pub const fn static_visible(visible: bool) -> Self {
        Self {
            visible: FocusRingVisibility::Static(visible),
            color: ColorAtom::ThemeRef("accent"),
            width: 2.0,
            offset: 2.0,
            opacity: 10_000,
        }
    }

    /// Use a concrete ring colour.
    #[must_use]
    pub const fn color(mut self, color: Rgba8) -> Self {
        self.color = ColorAtom::Rgba8(color);
        self
    }

    /// Use a theme token for the ring colour.
    #[must_use]
    pub const fn theme_color(mut self, token: &'static str) -> Self {
        self.color = ColorAtom::ThemeRef(token);
        self
    }

    /// Set ring stroke width in logical pixels.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the gap between the widget rect and the ring.
    #[must_use]
    pub const fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    /// Set opacity encoded as `value * 10000`.
    #[must_use]
    pub const fn opacity(mut self, opacity: u16) -> Self {
        debug_assert!(opacity <= 10_000);
        self.opacity = opacity;
        self
    }

    /// Whether the ring should be emitted this frame.
    #[must_use]
    pub fn visible(&self) -> bool {
        match self.visible {
            FocusRingVisibility::Static(visible) => visible,
            FocusRingVisibility::Reactive(visible) => visible.get(),
        }
    }

    /// Ring colour atom.
    #[must_use]
    pub const fn color_atom(&self) -> ColorAtom {
        self.color
    }

    /// Stroke width in logical pixels.
    #[must_use]
    pub const fn stroke_width(&self) -> f32 {
        self.width
    }

    /// Gap between widget rect and ring.
    #[must_use]
    pub const fn offset_px(&self) -> f32 {
        self.offset
    }

    /// Opacity encoded as `value * 10000`.
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
