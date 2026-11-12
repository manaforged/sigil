use super::*;
use crate::kernel::Cx;
use crate::reactive::Owner;

#[test]
fn empty_grid_has_no_rows() {
    Owner::scope("heatmap-empty", || {
        let mut cx = Cx::new_root();
        let root = cx.add(heatmap(0, 0, |_, _| Color::rgb(0, 0, 0)));
        assert_eq!(cx.child_count(root), 0);
    });
}

#[test]
fn a_grid_is_one_image_node() {
    Owner::scope("heatmap-shape", || {
        let mut cx = Cx::new_root();
        let root = cx.add(heatmap(100, 100, |_, _| Color::rgb(0, 0, 0)));
        assert_eq!(cx.child_count(root), 1, "one image for the whole grid");
        let picture = cx.nth_child(root, 0).and_then(|id| cx.get(id));
        assert!(matches!(
            picture.map(|node| &node.kind),
            Some(crate::kernel::NodeType::Image { .. })
        ));
    });
}

#[test]
fn color_runs_once_per_cell_at_build() {
    Owner::scope("heatmap-coords", || {
        let seen = std::cell::RefCell::new(Vec::new());
        let mut cx = Cx::new_root();
        let _ = cx.add(heatmap(5, 6, |x, y| {
            seen.borrow_mut().push((x, y));
            Color::rgb(0, 0, 0)
        }));
        let mut seen = seen.into_inner();
        seen.sort_unstable();
        let mut expected: Vec<(usize, usize)> =
            (0..6).flat_map(|y| (0..5).map(move |x| (x, y))).collect();
        expected.sort_unstable();
        assert_eq!(seen, expected, "color() called once per cell at build time");
    });
}
