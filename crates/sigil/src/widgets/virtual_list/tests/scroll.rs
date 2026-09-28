use super::*;

#[test]
fn build_materialises_only_visible_window() {
    Runtime::scope("sigil-virtual-list-visible-window", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..1000).collect();
        let root = cx.add(virtual_list(items, |_, _| container()));
        cx.flush_reactive_children();

        assert_eq!(cx.child_count(bound_visible(&cx, root)), 30);
    });
}

#[test]
fn overscan_zero_mounts_the_geometric_page() {
    Runtime::scope("sigil-virtual-list-overscan-zero", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..1000).collect();
        let root = cx.add(virtual_list(items, |_, _| container()).overscan(0));
        cx.flush_reactive_children();

        assert_eq!(cx.child_count(bound_visible(&cx, root)), 15);
    });
}

#[test]
fn overscan_zero_bind_mounts_the_geometric_page() {
    Runtime::scope("sigil-virtual-list-overscan-zero-bind", || {
        let items = signal((0..1000).collect::<Vec<usize>>());
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, |_, _| container())
                .overscan(0)
                .row_height(32.0)
                .viewport_height(480.0),
        );
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(bound_visible(&cx, root)), 15);
    });
}

#[test]
fn scroll_to_end_seeds_signal_at_bottom() {
    Runtime::scope("sigil-virtual-list-scroll-to-end", || {
        let scroll_y = crate::reactive::signal(0.0_f32);
        let items: Vec<usize> = (0..100).collect();
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .scroll_to_end(),
        );
        assert_eq!(scroll_y.get(), 2800.0);
    });
}

#[test]
fn scroll_into_view_seeds_signal_at_row_position() {
    Runtime::scope("sigil-virtual-list-scroll-into-view", || {
        let scroll_y = crate::reactive::signal(0.0_f32);
        let items: Vec<usize> = (0..100).collect();
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .scroll_into_view(50),
        );
        assert_eq!(scroll_y.get(), 1500.0);
    });
}

#[test]
fn auto_pin_to_bottom_no_op_for_static_items() {
    Runtime::scope("sigil-virtual-list-auto-pin", || {
        let scroll_y = crate::reactive::signal(0.0_f32);
        let items: Vec<usize> = (0..10).collect();
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .auto_pin_to_bottom(true),
        );
        assert_eq!(scroll_y.get(), 0.0);
    });
}

#[test]
fn bound_scroll_to_end_seeds_from_live_count() {
    Runtime::scope("sigil-vlist-bind-scroll-to-end", || {
        let scroll_y = signal(0.0_f32);
        let items = signal((0..100_u32).collect::<Vec<_>>());
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list_bind(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .scroll_to_end(),
        );
        assert_eq!(scroll_y.get(), 2800.0);
    });
}

#[test]
fn auto_pin_follows_grow_when_already_at_bottom() {
    Runtime::scope("sigil-vlist-auto-pin-follow", || {
        let scroll_y = signal(0.0_f32);
        let items = signal((0..100_u32).collect::<Vec<_>>());
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list_bind(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .scroll_to_end()
                .auto_pin_to_bottom(true),
        );
        assert_eq!(scroll_y.get(), 2800.0);
        items.set((0..120_u32).collect());
        assert_eq!(scroll_y.get(), 3400.0);
    });
}

#[test]
fn auto_pin_leaves_scroll_when_user_scrolled_up() {
    Runtime::scope("sigil-vlist-auto-pin-hold", || {
        let scroll_y = signal(0.0_f32);
        let items = signal((0..100_u32).collect::<Vec<_>>());
        let mut cx = Cx::new_root();
        let _ = cx.add(
            virtual_list_bind(items, |_, _| container())
                .row_height(30.0)
                .viewport_height(200.0)
                .scroll_bind(scroll_y)
                .auto_pin_to_bottom(true),
        );
        assert_eq!(scroll_y.get(), 0.0);
        items.set((0..120_u32).collect());
        assert_eq!(scroll_y.get(), 0.0);
    });
}
