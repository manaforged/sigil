use super::*;

impl OpenMargin {
    /// Zero, idle state. Use this when the parent has separation
    /// (padding/border on the top edge) — collapse cannot cross.
    pub(crate) const fn idle() -> Self {
        Self {
            value: 0.0,
            active: true,
        }
    }

    /// Carry an explicit margin into the collapse context (parent-first
    /// child case: the parent's own top margin is the seed).
    pub(crate) const fn seed(value: f32) -> Self {
        Self {
            value,
            active: true,
        }
    }
}

#[test]
fn positive_positive_takes_max() {
    assert_eq!(collapse(10.0, 20.0), 20.0);
    assert_eq!(collapse(30.0, 5.0), 30.0);
}

#[test]
fn negative_negative_takes_min() {
    assert_eq!(collapse(-10.0, -20.0), -20.0);
    assert_eq!(collapse(-3.0, -1.0), -3.0);
}

#[test]
fn mixed_sums() {
    assert_eq!(collapse(10.0, -4.0), 6.0);
    assert_eq!(collapse(-15.0, 20.0), 5.0);
}

#[test]
fn open_seed_then_absorb_takes_max() {
    let mut o = OpenMargin::seed(8.0);
    let gap = o.absorb_top(12.0);
    assert!((gap - 12.0).abs() < f32::EPSILON);
}

#[test]
fn open_idle_collapses_to_top() {
    let mut o = OpenMargin::idle();
    let gap = o.absorb_top(7.0);
    assert!((gap - 7.0).abs() < f32::EPSILON);
}

#[test]
fn open_sealed_uses_top_raw() {
    let mut o = OpenMargin::sealed();
    let gap = o.absorb_top(9.0);
    assert!((gap - 9.0).abs() < f32::EPSILON);
}

#[test]
fn emit_bottom_then_absorb_collapses() {
    let mut o = OpenMargin::sealed();
    o.emit_bottom(5.0);
    let gap = o.absorb_top(8.0);
    assert!((gap - 8.0).abs() < f32::EPSILON);
}
