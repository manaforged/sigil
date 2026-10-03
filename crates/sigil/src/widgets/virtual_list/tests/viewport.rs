use super::*;

#[test]
fn viewport_fill_sets_root_dimensions_to_fill() {
    use crate::layout::Dimension;
    Runtime::scope("sigil-virtual-list-viewport-fill", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..10).collect();
        let root = cx.add(virtual_list(items, |_, _| container()).viewport_fill());
        let style = cx
            .get(root)
            .expect("root node")
            .get_extension::<crate::layout::LayoutStyle>()
            .expect("root has LayoutStyle")
            .clone();
        assert_eq!(style.data().size.width, Dimension::Fill);
        assert_eq!(style.data().size.height, Dimension::Fill);
    });
}

#[test]
fn viewport_fill_tracks_parent_pane_measured_size() {
    use crate::geometry::{LogicalSize, Size};
    use crate::kernel::TextContent;
    use crate::layout::LayoutBuilderExt;
    use crate::layout::{LayoutEngine, LayoutStyle, TextMeasure};

    struct StubMeasure;
    impl TextMeasure for StubMeasure {
        fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
            LogicalSize::new(0.0, 0.0)
        }
    }

    Runtime::scope("sigil-virtual-list-viewport-fill-layout", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..50).collect();
        let parent = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, 300.0))
                .child(
                    virtual_list(items, |_, _| container())
                        .row_height(24.0)
                        .viewport_fill(),
                ),
        );
        let viewport = cx.first_child(parent).expect("virtual_list root");

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, parent, Size::fixed(800.0, 600.0))
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
    Runtime::scope("sigil-virtual-list-select", || {
        let last_index: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let last_clone = Rc::clone(&last_index);

        let items: Vec<usize> = (0..20).collect();
        let mut builder = virtual_list(items, |_, _| container())
            .row_height(30.0)
            .viewport_height(200.0)
            .selection(SelectionMode::Single);

        let selection = builder.selection_state();
        let builder = builder.change(move |i| {
            last_clone.set(Some(i));
        });

        let mut cx = Cx::new_root();
        let _ = cx.add(builder);

        selection.update(|state| state.click(7));
        assert_eq!(last_index.get(), Some(7));
    });
}
