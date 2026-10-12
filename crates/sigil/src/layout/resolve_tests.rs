use super::*;
use crate::layout::style_data::{BoxSizing, Dimension};

impl Edges {
    pub(crate) const fn inline(self) -> f32 {
        self.left + self.right
    }

    pub(crate) const fn block(self) -> f32 {
        self.top + self.bottom
    }
}

#[test]
fn resolve_dim_handles_px_percent_and_auto() {
    assert_eq!(resolve_dim(Dimension::Px(120.0), 500.0), Some(120.0));
    assert_eq!(resolve_dim(Dimension::Percent(0.25), 800.0), Some(200.0));
    assert!(resolve_dim(Dimension::Auto, 100.0).is_none());
}

#[test]
fn clamp_applies_max_then_min_floor() {
    assert_eq!(clamp(50.0, Some(60.0), Some(100.0)), 60.0);
    assert_eq!(clamp(200.0, Some(60.0), Some(100.0)), 100.0);
    assert_eq!(clamp(80.0, Some(60.0), Some(100.0)), 80.0);
    assert_eq!(clamp(80.0, Some(120.0), Some(100.0)), 120.0);
}

#[test]
fn box_sizing_decides_chrome_addition() {
    let px = Dimension::Px(100.0);
    assert_eq!(
        dim_to_border_box(px, 0.0, 20.0, BoxSizing::BorderBox),
        Some(100.0)
    );
    assert_eq!(
        dim_to_border_box(px, 0.0, 20.0, BoxSizing::ContentBox),
        Some(120.0)
    );
}
