use std::rc::Rc;

use super::palette::Hue;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum Orient {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum Dot {
    #[default]
    Disc,
    Ring,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Fit {
    pub(super) domain: Option<(f32, f32)>,
    pub(super) zero: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Scales {
    pub(super) orient: Orient,
    pub(super) dot: Dot,
    pub(super) x: Fit,
    pub(super) y: Fit,
    pub(super) time: Option<f64>,
    pub(super) x_title: Option<String>,
    pub(super) y_title: Option<String>,
    pub(super) slices: Option<Rc<[Hue]>>,
    pub(super) plot_height: Option<f32>,
}

impl Fit {
    pub(super) fn zero_or(self, fallback: bool) -> bool {
        self.zero.unwrap_or(fallback)
    }
}
