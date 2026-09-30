use crate::layout::{IntoPx, LayoutStyle};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutOverrides {
    full: Option<LayoutStyle>,
    width: Option<f32>,
    height: Option<f32>,
    grow: Option<f32>,
}

impl LayoutOverrides {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            full: None,
            width: None,
            height: None,
            grow: None,
        }
    }

    pub fn layout(&mut self, style: LayoutStyle) {
        self.full = Some(style);
    }

    pub const fn width(&mut self, px: f32) {
        self.width = Some(px);
    }

    pub const fn height(&mut self, px: f32) {
        self.height = Some(px);
    }

    pub const fn grow(&mut self, factor: f32) {
        self.grow = Some(factor);
    }

    #[must_use]
    pub const fn width_hint(&self) -> Option<f32> {
        self.width
    }

    #[must_use]
    pub const fn height_hint(&self) -> Option<f32> {
        self.height
    }

    #[must_use]
    pub fn apply(self, base: LayoutStyle) -> LayoutStyle {
        let mut style = base;
        if let Some(full) = &self.full {
            style.merge(full);
        }
        if let Some(width) = self.width {
            style = style.width(width);
            if let crate::layout::Dimension::Px(floor) = style.0.min_size.width
                && floor > width
            {
                let height = style.0.min_size.height;
                style = style.min_size(width, height);
            }
        }
        if let Some(height) = self.height {
            style = style.height(height);
        }
        if let Some(grow) = self.grow {
            style = style.grow(grow);
        }
        style
    }
}

pub trait WidgetLayoutExt: Sized {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides;

    #[must_use]
    fn layout(mut self, style: LayoutStyle) -> Self {
        LayoutOverrides::layout(self.layout_overrides_mut(), style);
        self
    }

    #[must_use]
    fn width(mut self, width: impl IntoPx) -> Self {
        LayoutOverrides::width(self.layout_overrides_mut(), width.into_px());
        self
    }

    #[must_use]
    fn height(mut self, height: impl IntoPx) -> Self {
        LayoutOverrides::height(self.layout_overrides_mut(), height.into_px());
        self
    }

    #[must_use]
    fn grow(mut self, factor: f32) -> Self {
        LayoutOverrides::grow(self.layout_overrides_mut(), factor);
        self
    }
}

#[cfg(test)]
#[path = "layout_ext_tests.rs"]
mod tests;
