use super::*;

#[test]
fn rows_built_by_a_strip_rerun_keep_their_signals() {
    Runtime::scope("sigil-vlist-rerun-signals", || {
        let items = signal((0..1_000u32).collect::<Vec<_>>());
        let scroll_y = signal(0.0_f32);
        let made: Rc<Made> = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&made);
        let mut cx = Cx::new_root();
        let _root = cx.add(
            virtual_list_bind(items, move |index, _| {
                sink.borrow_mut().push((index, signal(index)));
                container().label(format!("row {index}"))
            })
            .scroll_bind(scroll_y)
            .row_height(20.0)
            .viewport_height(100.0),
        );
        cx.flush_reactive_children();
        let initial = made.borrow().len();
        scroll_y.set(200.0);
        cx.flush_reactive_children();
        let after_first = made.borrow().len();
        assert!(after_first > initial, "scrolling must build new rows");
        scroll_y.set(220.0);
        cx.flush_reactive_children();
        for (index, live) in made
            .borrow()
            .iter()
            .skip(initial)
            .take(after_first - initial)
        {
            assert_eq!(
                live.peek(),
                *index,
                "row {index} lost its signal after a rerun"
            );
        }
    });
}

#[test]
fn scrolling_rebuilds_rows_at_most_once_per_block() {
    Runtime::scope("sigil-vlist-band", || {
        let items = signal((0..1_000u32).collect::<Vec<_>>());
        let scroll_y = signal(0.0_f32);
        let built = Rc::new(std::cell::Cell::new(0_usize));
        let sink = Rc::clone(&built);
        let mut cx = Cx::new_root();
        let _root = cx.add(
            virtual_list_bind(items, move |index, _| {
                sink.set(sink.get() + 1);
                container().label(format!("row {index}"))
            })
            .scroll_bind(scroll_y)
            .row_height(20.0)
            .viewport_height(400.0),
        );
        cx.flush_reactive_children();
        let mut rebuilds = 0;
        let mut last = built.get();
        for row in 1_u8..=4 {
            scroll_y.set(f32::from(row) * 20.0);
            cx.flush_reactive_children();
            if built.get() != last {
                rebuilds += 1;
                last = built.get();
            }
        }
        assert!(
            rebuilds <= 1,
            "four rows of scroll rebuilt {rebuilds} times"
        );
        scroll_y.set(2_000.0);
        cx.flush_reactive_children();
        assert!(
            built.get() > last,
            "leaving the mounted blocks must mount new rows"
        );
    });
}

#[test]
fn scrolled_rows_land_at_their_document_position() {
    use crate::hal::{LogicalSize, Size};
    use crate::kernel::TextContent;
    use crate::layout::prelude::LayoutBuilderExt;
    use crate::layout::{LayoutEngine, TextMeasure};

    struct StubMeasure;
    impl TextMeasure for StubMeasure {
        fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
            LogicalSize::new(0.0, 0.0)
        }
    }

    Runtime::scope("sigil-virtual-list-scrolled-position", || {
        let items = signal((0..1_000u32).collect::<Vec<_>>());
        let scroll_y = signal(0.0_f32);
        let tick = signal(0_u32);
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, move |index, _| {
                container()
                    .layout(crate::layout::LayoutStyle::block().fixed_size(800.0, 24.0))
                    .label(format!("row {index}"))
                    .child(crate::kernel::text_live(move || {
                        format!("{index}:{}", tick.get())
                    }))
            })
            .key_by(|index, _| index as u64)
            .scroll_bind(scroll_y)
            .row_height(24.0)
            .viewport_height(1_000.0)
            .viewport_width(800.0),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        let size = Size::fixed(800.0, 1_000.0);
        layout.compute(&mut cx, root, size).expect("layout");
        for frame in 1..=30_u32 {
            tick.set(frame);
            if frame % 2 == 0 {
                scroll_y.set(frame as f32 * 36.0);
            }
            layout.compute(&mut cx, root, size).expect("layout");
            let scrolled = (frame - frame % 2) as f32 * 36.0;
            let expected = (scrolled / 24.0).floor() as usize;
            let top = top_row(&cx, &layout, root);
            assert_eq!(
                top,
                Some(expected),
                "frame {frame}: row at the viewport top"
            );
        }
    });
}
