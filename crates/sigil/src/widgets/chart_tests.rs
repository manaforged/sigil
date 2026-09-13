#![allow(clippy::unwrap_used, clippy::float_cmp)]

use super::*;
use crate::kernel::IntoNode;
use crate::reactive::Runtime;

#[test]
fn nice_max_steps_are_1_2_5() {
    assert_eq!(nice_max(3.0), 5.0);
    assert_eq!(nice_max(12.0), 20.0);
    assert_eq!(nice_max(0.0), 1.0);
}

#[test]
fn empty_chart_builds() {
    Runtime::scope("chart-empty", || {
        let mut cx = Cx::new_root();
        let id = chart([]).build(&mut cx);
        assert!(cx.get(id).is_some());
    });
}

#[test]
fn bar_chart_builds() {
    Runtime::scope("chart-bar", || {
        let mut cx = Cx::new_root();
        let id = chart([4.0, 8.0, 3.0, 10.0])
            .labels(["Mon", "Tue", "Wed", "Thu"])
            .build(&mut cx);
        assert!(cx.get(id).is_some());
        assert!(!cx.get(id).unwrap().children().is_empty());
    });
}

#[test]
fn line_chart_builds() {
    Runtime::scope("chart-line", || {
        let mut cx = Cx::new_root();
        let id = chart([1.0, 4.0, 2.0, 8.0]).line().build(&mut cx);
        assert!(cx.get(id).is_some());
    });
}
