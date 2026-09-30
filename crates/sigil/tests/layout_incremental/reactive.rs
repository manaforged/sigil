use super::*;

#[test]
fn reactive_text_change_invalidates_cache() {
    Runtime::scope("layout_reactive_text_cache", || {
        let value = signal(String::from("tiny"));
        let mut cx = Cx::new_root();
        let root = cx.add(text(value));
        let mut engine = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));

        engine
            .compute(&mut cx, root, Size::max_content())
            .expect("initial text compute succeeds");
        let generation = engine.generation();
        let initial_width = layout_rect(&cx, root).width;

        value.set(String::from("much wider"));
        engine
            .compute(&mut cx, root, Size::max_content())
            .expect("changed text compute succeeds");

        assert_ne!(engine.generation(), generation);
        assert!(layout_rect(&cx, root).width > initial_width);
    });
}

#[test]
fn reactive_display_none_then_empty_shows_box() {
    Runtime::scope("layout_reactive_display_none_empty", || {
        let show = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(40.0, 20.0))
                .visible_when(show),
        );
        let mut engine = stub_engine();

        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("hidden compute succeeds");
        assert_eq!(layout_rect(&cx, root).width.to_bits(), 0.0_f32.to_bits());

        show.set(true);
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("shown compute succeeds");
        assert_eq!(layout_rect(&cx, root).width.to_bits(), 40.0_f32.to_bits());
        assert_eq!(layout_rect(&cx, root).height.to_bits(), 20.0_f32.to_bits());
    });
}
