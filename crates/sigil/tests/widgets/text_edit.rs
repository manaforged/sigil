use super::*;

#[test]
fn text_field_shift_arrow_selects_and_cut_clears_value() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-selection", || {
        sigil::clipboard::install(CutMemory(RefCell::new(None)));
        let value = signal(String::from("hello"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, named_key(Key::Home, PhysicalKey::Home));
        for _ in 0..5 {
            router.route(
                &mut cx,
                &layout,
                shift_arrow(Key::ArrowRight, PhysicalKey::ArrowRight),
            );
        }
        router.route(&mut cx, &layout, ctrl_key("x", PhysicalKey::KeyX));
        assert_eq!(value.get(), "");
    });
}

#[test]
fn text_field_ctrl_a_selects_all_and_delete_clears() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-select-all", || {
        let value = signal(String::from("hello world"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, ctrl_key("a", PhysicalKey::KeyA));
        router.route(&mut cx, &layout, backspace_down());
        assert_eq!(value.get(), "");
    });
}

#[test]
fn text_field_undo_restores_previous_value() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-undo", || {
        let value = signal(String::new());
        let builder = text_field(value);
        let mut cx = Cx::new_root();
        let root = cx.add(builder);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("a", PhysicalKey::KeyA));
        router.route(&mut cx, &layout, type_char("b", PhysicalKey::KeyB));
        assert_eq!(value.get(), "ab");

        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "a", "undo should restore value before 'b'");

        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "", "second undo should restore empty");
    });
}

#[test]
fn text_field_redo_reapplies_undone_change() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-redo", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("x", PhysicalKey::KeyX));
        assert_eq!(value.get(), "x");

        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "");

        router.route(&mut cx, &layout, ctrl_shift_key("Z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "x", "redo should reapply 'x'");
    });
}

#[test]
fn text_field_caret_visible_resets_on_keystroke() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-caret-blink", || {
        let value = signal(String::new());
        let mut builder = text_field(value);
        let caret_visible = builder.caret_visible();
        let mut cx = Cx::new_root();
        let root = cx.add(builder);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        assert!(caret_visible.get(), "starts visible");

        caret_visible.set(false);
        assert!(!caret_visible.get());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("k", PhysicalKey::KeyK));
        assert!(
            caret_visible.get(),
            "keystroke resets caret_visible to true"
        );
    });
}

#[test]
fn text_field_scroll_offset_advances_past_visible_window() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-scroll", || {
        set_text_geometry(Rc::new(StubGeometry));
        let value = signal(String::new());
        let mut builder = text_field(value);
        let scroll_offset = builder.scroll_offset();
        let mut cx = Cx::new_root();
        let root = cx.add(builder);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        for ch in "abcdefghijklmnopqrstuvwxyz".chars() {
            let key_str = ch.to_string();
            router.route(
                &mut cx,
                &layout,
                type_char_owned(key_str, PhysicalKey::KeyA),
            );
        }
        assert!(
            scroll_offset.get() > 0.0,
            "scroll_offset should advance after long input, got {}",
            scroll_offset.get()
        );
    });
}
