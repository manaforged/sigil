use super::*;

#[test]
fn button_label_accepts_a_signal() {
    sigil::reactive::Runtime::scope("sigil-widget-button-signal-label", || {
        let caption = signal(String::from("Save"));
        let mut cx = Cx::new_root();
        let root = cx.add(button(caption));
        let node = cx.get(root).expect("button");
        assert_eq!(node.a11y.role, Role::Button);
        let child_id = *node.children().first().expect("label text");
        let child = cx.get(child_id).expect("label text");
        match &child.kind {
            NodeType::Text { content, .. } => match content {
                TextContent::Reactive(value) => assert_eq!(value.get(), "Save"),
                TextContent::Static(value) => panic!("expected reactive label, got {value}"),
                TextContent::Compute(_) => panic!("expected reactive label, got compute"),
            },
            other => panic!("expected text child, got {other:?}"),
        }
        caption.set(String::from("Saving"));
        match &cx.get(child_id).expect("label text").kind {
            NodeType::Text {
                content: TextContent::Reactive(value),
                ..
            } => assert_eq!(value.get(), "Saving"),
            other => panic!("expected reactive label after set, got {other:?}"),
        }
    });
}

#[test]
fn button_is_focusable_and_activates_on_enter() {
    sigil::reactive::Runtime::scope("sigil-widget-button-enter", || {
        let hits = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(button("Queue run").primary().activate({
            let hits = Rc::clone(&hits);
            move || hits.set(hits.get() + 1)
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, enter_down());

        assert_eq!(hits.get(), 1);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::Button);
        assert!(node.a11y.focusable);
        assert!(node.get_extension::<FocusRing>().unwrap().visible());
    });
}

#[test]
fn disabled_button_does_not_focus_or_activate() {
    sigil::reactive::Runtime::scope("sigil-widget-button-disabled", || {
        let hits = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(button("Disabled").enabled(false).activate({
            let hits = Rc::clone(&hits);
            move || hits.set(hits.get() + 1)
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, enter_down());

        assert_eq!(hits.get(), 0);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hittable);
    });
}

#[test]
fn button_pointer_activation_requires_press_inside() {
    sigil::reactive::Runtime::scope("sigil-widget-button-pointer", || {
        let hits = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(button("Queue run").activate({
            let hits = Rc::clone(&hits);
            move || hits.set(hits.get() + 1)
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_move(500.0, 500.0));
        router.route(&mut cx, &layout, pointer_up(500.0, 500.0));

        assert_eq!(hits.get(), 1);
    });
}

#[test]
fn label_accepts_reactive_text() {
    sigil::reactive::Runtime::scope("sigil-widget-label-reactive", || {
        let value = signal(String::from("Ready"));
        let mut cx = Cx::new_root();
        let root = cx.add(text(value).nowrap());
        let node = cx.get(root).unwrap();
        let is_reactive_text = matches!(&node.kind, NodeType::Text { content, .. } if content.as_static_str().is_none());
        assert!(is_reactive_text);
    });
}
