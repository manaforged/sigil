use super::*;

#[test]
fn items_bind_rows_follow_the_signal() {
    Runtime::scope("sigil-vlist-items-bind", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list_bind(items, |_, _| container()));
        for (next, expected) in [
            (None, 3),
            (Some(vec!["x", "y"]), 2),
            (Some(Vec::new()), 0),
            (Some(vec!["p", "q", "r"]), 3),
            (Some(Vec::new()), 0),
        ] {
            if let Some(next) = next {
                items.set(next);
            }
            cx.flush_reactive_children();
            assert_eq!(cx.child_count(bound_visible(&cx, root)), expected);
        }
    });
}

#[test]
fn items_bind_static_path_still_works() {
    let empty = mounted(|| virtual_list_bind(signal(Vec::<&str>::new()), |_, _| container()));
    assert_eq!(empty, 0);
    assert_eq!(
        mounted(|| virtual_list((0..5).collect::<Vec<usize>>(), |_, _| container())),
        5
    );
}

#[test]
fn items_bind_window_advances_in_place_on_scroll() {
    Runtime::scope("sigil-vlist-bind-scroll-advance", || {
        let items = signal((0..10_000).collect::<Vec<usize>>());
        let scroll_y = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, |index, _| container().label(format!("row {index}")))
                .scroll_bind(scroll_y)
                .row_height(20.0)
                .viewport_height(100.0),
        );
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(bound_visible(&cx, root)), 10);
        assert_eq!(first_row_label(&cx, root), Some("row 0".to_string()));

        scroll_y.set(4_000.0);
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(bound_visible(&cx, root)), 15);
        assert_eq!(
            first_row_label(&cx, root),
            Some("row 195".to_string()),
            "scroll_y write must advance the padded window in place"
        );
    });
}

#[test]
fn scroll_one_row_keeps_overlapping_node() {
    Runtime::scope("sigil-vlist-keyed-reuse", || {
        let items = signal((0..1_000u32).collect::<Vec<_>>());
        let scroll_y = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, |index, _| container().label(format!("row {index}")))
                .scroll_bind(scroll_y)
                .row_height(20.0)
                .viewport_height(100.0),
        );
        cx.flush_reactive_children();
        let before = bound_visible(&cx, root);
        let keep = cx.nth_child(before, 1).expect("row 1");
        scroll_y.set(20.0);
        cx.flush_reactive_children();
        let after = bound_visible(&cx, root);
        let still = cx.nth_child(after, 1).expect("row 1 still mounted");
        assert_eq!(still, keep, "overlapping row must keep NodeId");
        assert_eq!(first_row_label(&cx, root), Some("row 0".to_string()));
    });
}

#[test]
fn identified_reorder_keeps_row_nodes() {
    #[derive(Clone, Copy, PartialEq)]
    struct Row {
        id: u8,
        name: &'static str,
    }
    impl Identifiable for Row {
        type Id = u8;
        fn id(&self) -> Self::Id {
            self.id
        }
    }

    Runtime::scope("sigil-vlist-identified-reorder", || {
        let items = signal(vec![Row { id: 1, name: "a" }, Row { id: 2, name: "b" }]);
        let mut cx = Cx::new_root();
        let root = cx.add(virtual_list(items, |_, row| container().label(row.name)).identified());
        cx.flush_reactive_children();
        let visible = bound_visible(&cx, root);
        let before = [
            cx.nth_child(visible, 0).expect("row a"),
            cx.nth_child(visible, 1).expect("row b"),
        ];

        items.set(vec![Row { id: 2, name: "b" }, Row { id: 1, name: "a" }]);
        cx.flush_reactive_children();
        let reordered = bound_visible(&cx, root);
        let after = [
            cx.nth_child(reordered, 0).expect("row b"),
            cx.nth_child(reordered, 1).expect("row a"),
        ];
        assert_eq!(after, [before[1], before[0]]);
        assert_eq!(first_row_label(&cx, root), Some("b".to_string()));
    });
}
