use super::*;

#[test]
fn collapse_takes_max_min_or_sum_by_sign() {
    assert_eq!(collapse(10.0, 20.0), 20.0);
    assert_eq!(collapse(30.0, 5.0), 30.0);
    assert_eq!(collapse(-10.0, -20.0), -20.0);
    assert_eq!(collapse(-3.0, -1.0), -3.0);
    assert_eq!(collapse(10.0, -4.0), 6.0);
    assert_eq!(collapse(-15.0, 20.0), 5.0);
}

#[test]
fn open_margin_absorbs_top() {
    let open = |value| OpenMargin {
        value,
        active: true,
    };
    assert_eq!(open(8.0).absorb_top(12.0), 12.0);
    assert_eq!(open(0.0).absorb_top(7.0), 7.0);
    assert_eq!(OpenMargin::sealed().absorb_top(9.0), 9.0);
    let mut o = OpenMargin::sealed();
    o.emit_bottom(5.0);
    assert_eq!(o.absorb_top(8.0), 8.0);
}
