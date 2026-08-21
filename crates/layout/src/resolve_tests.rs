use super::*;
use crate::style_data::{BoxSizing, Dimension};

impl Edges {
    /// Sum of the inline-axis edges (left + right).
    pub(crate) const fn inline(self) -> f32 {
        self.left + self.right
    }

    /// Sum of the block-axis edges (top + bottom).
    pub(crate) const fn block(self) -> f32 {
        self.top + self.bottom
    }
}

#[test]
fn dim_px_resolves_directly() {
    assert_eq!(resolve_dim(Dimension::Px(120.0), 500.0), Some(120.0));
}

#[test]
fn dim_percent_resolves_against_basis() {
    let v = resolve_dim(Dimension::Percent(0.25), 800.0).unwrap();
    assert!((v - 200.0).abs() < f32::EPSILON);
}

#[test]
fn dim_auto_returns_none() {
    assert!(resolve_dim(Dimension::Auto, 100.0).is_none());
}

#[test]
fn clamp_applies_min_then_max() {
    assert_eq!(clamp(50.0, Some(60.0), Some(100.0)), 60.0);
    assert_eq!(clamp(200.0, Some(60.0), Some(100.0)), 100.0);
    assert_eq!(clamp(80.0, Some(60.0), Some(100.0)), 80.0);
}

#[test]
fn clamp_min_floors_above_max() {
    // CSS: min wins when min > max.
    assert_eq!(clamp(80.0, Some(120.0), Some(100.0)), 120.0);
}

#[test]
fn border_box_skips_chrome_addition() {
    let v = dim_to_border_box(Dimension::Px(100.0), 0.0, 20.0, BoxSizing::BorderBox);
    assert_eq!(v, Some(100.0));
}

#[test]
fn content_box_adds_chrome() {
    let v = dim_to_border_box(Dimension::Px(100.0), 0.0, 20.0, BoxSizing::ContentBox);
    assert_eq!(v, Some(120.0));
}
