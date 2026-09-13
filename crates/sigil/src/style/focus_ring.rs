use crate::reactive::Signal;

use crate::style::{ColorAtom, Rgba8};

#[derive(Clone, Debug)]
pub struct FocusRing {
    visible: FocusRingVisibility,
    color: ColorAtom,
    width: f32,
    offset: f32,
    opacity: u16,
}

impl FocusRing {
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

    #[must_use]
    pub const fn color(mut self, color: Rgba8) -> Self {
        self.color = ColorAtom::Rgba8(color);
        self
    }

    #[must_use]
    pub const fn theme_color(mut self, token: &'static str) -> Self {
        self.color = ColorAtom::ThemeRef(token);
        self
    }

    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub const fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
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
    pub const fn stroke_width(&self) -> f32 {
        self.width
    }

    #[must_use]
    pub const fn offset_px(&self) -> f32 {
        self.offset
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
