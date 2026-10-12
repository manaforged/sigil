use super::*;

fn seeded<T: 'static, R: RowRender<T>>(
    list: VirtualListBuilder<T, R>,
    scroll_y: Signal<f32>,
) -> Cx {
    let mut cx = Cx::new_root();
    let _ = cx.add(
        list.row_height(30.0)
            .viewport_height(200.0)
            .scroll_bind(scroll_y),
    );
    cx
}

fn hundred() -> Vec<usize> {
    (0..100).collect()
}

#[test]
fn build_materialises_only_visible_window() {
    assert_eq!(
        mounted(|| virtual_list((0..1000).collect::<Vec<usize>>(), |_, _| container())),
        30
    );
}

#[test]
fn overscan_zero_mounts_the_geometric_page() {
    let items = || (0..1000).collect::<Vec<usize>>();
    assert_eq!(
        mounted(|| virtual_list(items(), |_, _| container()).overscan(0)),
        15
    );
    let bound = mounted(|| {
        virtual_list_bind(signal(items()), |_, _| container())
            .overscan(0)
            .row_height(32.0)
            .viewport_height(480.0)
    });
    assert_eq!(bound, 15);
}

#[test]
fn initial_scroll_seeds_the_signal() {
    Runtime::scope("sigil-virtual-list-initial-scroll", || {
        let end = signal(0.0_f32);
        let _end = seeded(
            virtual_list(hundred(), |_, _| container()).initial_scroll(ScrollAnchor::Bottom),
            end,
        );
        assert_eq!(end.get(), 2800.0);
        let row = signal(0.0_f32);
        let _row = seeded(
            virtual_list(hundred(), |_, _| container()).scroll_into_view(50),
            row,
        );
        assert_eq!(row.get(), 1500.0);
        let short = signal(0.0_f32);
        let _short = seeded(
            virtual_list((0..10).collect::<Vec<usize>>(), |_, _| container()).follow_end(true),
            short,
        );
        assert_eq!(short.get(), 100.0);
        let bound = signal(0.0_f32);
        let list = virtual_list_bind(signal(hundred()), |_, _| container());
        let _bound = seeded(list.initial_scroll(ScrollAnchor::Bottom), bound);
        assert_eq!(bound.get(), 2800.0);
    });
}

#[test]
fn auto_pin_follows_grow_only_while_at_bottom() {
    Runtime::scope("sigil-vlist-auto-pin", || {
        for (scrolled_up, expected) in [(false, 3400.0), (true, 0.0)] {
            let scroll_y = signal(0.0_f32);
            let items = signal(hundred());
            let _cx = seeded(
                virtual_list_bind(items, |_, _| container()).follow_end(true),
                scroll_y,
            );
            assert_eq!(scroll_y.get(), 2800.0);
            if scrolled_up {
                scroll_y.set(0.0);
            }
            items.set((0..120).collect());
            assert_eq!(scroll_y.get(), expected);
        }
    });
}
