use super::*;
use crate::kernel::Cx;
use crate::reactive::Runtime;

fn rgba(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

#[test]
fn empty_grid_has_no_rows() {
    Runtime::scope("heatmap-empty", || {
        let mut cx = Cx::new_root();
        let root = cx.add(heatmap(0, 0, |_, _| rgba(0, 0, 0)));
        assert_eq!(cx.child_count(root), 0);
    });
}

#[test]
fn grid_shape_matches_dimensions() {
    Runtime::scope("heatmap-shape", || {
        let mut cx = Cx::new_root();
        let root = cx.add(heatmap(3, 4, |_, _| rgba(0, 0, 0)));
        assert_eq!(cx.child_count(root), 4, "one child per row");
        for r in 0..4 {
            let row = cx.nth_child(root, r).expect("row exists");
            assert_eq!(cx.child_count(row), 3, "three cells per row");
        }
    });
}

#[test]
fn closure_evaluated_eagerly_at_build() {
    Runtime::scope("heatmap-eager", || {
        use std::cell::Cell;
        let calls = Cell::new(0usize);
        let mut cx = Cx::new_root();
        let _ = cx.add(heatmap(5, 6, |_, _| {
            calls.set(calls.get() + 1);
            rgba(1, 2, 3)
        }));
        assert_eq!(
            calls.get(),
            30,
            "color() called once per cell at build time"
        );
    });
}

#[test]
fn coordinates_cover_full_grid() {
    Runtime::scope("heatmap-coords", || {
        use std::cell::RefCell;
        let seen: RefCell<Vec<(usize, usize)>> = RefCell::new(Vec::new());
        let mut cx = Cx::new_root();
        let _ = cx.add(heatmap(2, 3, |x, y| {
            seen.borrow_mut().push((x, y));
            rgba(0, 0, 0)
        }));
        let mut seen = seen.into_inner();
        seen.sort_unstable();
        let mut expected: Vec<(usize, usize)> =
            (0..3).flat_map(|y| (0..2).map(move |x| (x, y))).collect();
        expected.sort_unstable();
        assert_eq!(seen, expected);
    });
}
