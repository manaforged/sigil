use super::*;
use crate::widgets::Layout;

#[test]
fn fill_sets_root_dimensions_to_fill() {
    use crate::layout::Length;
    use crate::widgets::Widget;
    Owner::scope("sigil-virtual-list-viewport-fill", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..10).collect();
        let root = cx.add(virtual_list(items, |_, _| container()).fill());
        let style = cx
            .get(root)
            .expect("root node")
            .extension::<crate::layout::LayoutStyle>()
            .expect("root has LayoutStyle")
            .clone();
        assert_eq!(style.data().size.width, Length::Fill);
        assert_eq!(style.data().size.height, Length::Fill);
    });
}

#[test]
fn fill_tracks_parent_pane_measured_size() {
    use crate::geometry::Constraint;
    use crate::layout::LayoutStyle;
    use crate::widgets::Widget;

    Owner::scope("sigil-virtual-list-viewport-fill-layout", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..50).collect();
        let parent = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, 300.0))
                .child(
                    virtual_list(items, |_, _| container())
                        .row_height(24.0)
                        .fill(),
                ),
        );
        let viewport = cx.first_child(parent).expect("virtual_list root");

        let mut layout = engine();
        layout
            .compute(&mut cx, parent, Constraint::fixed(800.0, 600.0))
            .expect("layout");

        let parent_rect = layout.result(parent).expect("parent layout").rect();
        let viewport_rect = layout.result(viewport).expect("viewport layout").rect();
        assert!(
            (viewport_rect.width - parent_rect.width).abs() < 0.5,
            "viewport width {} should track parent width {}",
            viewport_rect.width,
            parent_rect.width
        );
        assert!(
            (viewport_rect.height - parent_rect.height).abs() < 0.5,
            "viewport height {} should track parent height {}",
            viewport_rect.height,
            parent_rect.height
        );
    });
}

#[test]
fn select_fires_with_selected_index() {
    use std::cell::Cell;
    use std::rc::Rc;
    Owner::scope("sigil-virtual-list-select", || {
        let last_index: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let last_clone = Rc::clone(&last_index);

        let items: Vec<usize> = (0..20).collect();
        let selection = signal(Selection::new(SelectionMode::Single));
        let builder = virtual_list(items, |_, _| container())
            .row_height(30.0)
            .viewport_height(200.0)
            .selection(SelectionMode::Single)
            .selected(selection)
            .change(move |i| {
                last_clone.set(Some(i));
            });

        let mut cx = Cx::new_root();
        let _ = cx.add(builder);

        selection.update(|state| state.select(7));
        assert_eq!(last_index.get(), Some(7));
    });
}
