use super::*;

#[test]
fn checkbox_click_flips_signal() {
    sigil::reactive::Runtime::scope("sigil-widget-checkbox-click", || {
        let checked = signal(false);
        let changes = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).label("Restock alerts").change({
            let changes = Rc::clone(&changes);
            move |next| {
                assert!(next);
                changes.set(changes.get() + 1);
            }
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));

        assert!(checked.get());
        assert_eq!(changes.get(), 1);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::CheckBox);
        assert!(node.a11y.checked.unwrap().get());
    });
}

#[test]
fn checkbox_keyboard_activation() {
    sigil::reactive::Runtime::scope("sigil-widget-checkbox-keyboard", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).label("Monitor"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, space_down());
        assert!(checked.get());

        router.route(&mut cx, &layout, enter_down());
        assert!(!checked.get());
    });
}

#[test]
fn checkbox_disabled_ignores_click() {
    sigil::reactive::Runtime::scope("sigil-widget-checkbox-disabled", || {
        let checked = signal(false);
        let disabled = signal(true);
        let changes = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(
            checkbox(checked)
                .label("Disabled")
                .disabled(disabled)
                .change({
                    let changes = Rc::clone(&changes);
                    move |_| changes.set(changes.get() + 1)
                }),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));

        assert!(!checked.get());
        assert_eq!(changes.get(), 0);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hittable);
    });
}

#[test]
fn checkbox_scene_emits_checked_and_unchecked_variants() {
    sigil::reactive::Runtime::scope("sigil-widget-checkbox-scene", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).label("Visual").tone(Tone::Primary));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let theme = Theme::default();
        let unchecked_colors = scene_rect_colors(&cx, root);
        assert!(unchecked_colors.contains(&theme.gray.step(1)));

        checked.set(true);
        let checked_colors = scene_rect_colors(&cx, root);
        assert!(checked_colors.contains(&theme.accent));
        assert!(
            emit_scene(&cx, root)
                .commands
                .iter()
                .any(|command| matches!(
                    command,
                    SceneCommand::Image(image) if image.opacity == 10_000
                )),
            "checked scene must paint the white vector check mark"
        );
    });
}

#[test]
fn toggle_click_flips_signal() {
    sigil::reactive::Runtime::scope("sigil-widget-toggle-click", || {
        let on = signal(false);
        let changes = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).label("Watch").change({
            let changes = Rc::clone(&changes);
            move |next| {
                assert!(next);
                changes.set(changes.get() + 1);
            }
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));

        assert!(on.get());
        assert_eq!(changes.get(), 1);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::Switch);
        assert!(node.a11y.checked.unwrap().get());
    });
}

#[test]
fn toggle_keyboard_activation() {
    sigil::reactive::Runtime::scope("sigil-widget-toggle-keyboard", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).label("Monitor"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, space_down());
        assert!(on.get());

        router.route(&mut cx, &layout, enter_down());
        assert!(!on.get());
    });
}

#[test]
fn toggle_scene_emits_on_and_off_variants() {
    sigil::reactive::Runtime::scope("sigil-widget-toggle-scene", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).label("Visual"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let theme = Theme::default();
        let off_colors = scene_rect_colors(&cx, root);
        let off_thumb_x =
            first_rect_x_for_color(&cx, root, theme.color(sigil::style::Token::Gray11));
        assert!(off_colors.contains(&theme.color(sigil::style::Token::Gray6)));

        on.set(true);
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout recomputes");
        for _ in 0..120 {
            sigil::animation_runtime::tick(1.0 / 60.0);
        }
        let on_colors = scene_rect_colors(&cx, root);
        let on_thumb_x = first_rect_x_for_color(&cx, root, theme.on_accent);
        assert!(on_colors.contains(&theme.accent));
        assert!(on_thumb_x > off_thumb_x);
    });
}

#[test]
fn toggle_and_checkbox_share_styled_interaction_path() {
    let interaction_src = include_str!("../../src/widgets/interaction.rs");
    let shared_src = include_str!("../../src/widgets/shared.rs");
    let checkbox_src = include_str!("../../src/widgets/checkbox.rs");
    let toggle_src = include_str!("../../src/widgets/toggle.rs");

    assert!(interaction_src.contains("pub fn style_state"));
    assert!(interaction_src.contains("pub fn release_pointer"));
    assert!(shared_src.contains("interaction.release_pointer"));
    assert!(shared_src.contains("interaction.activate_key"));
    assert!(shared_src.contains("binary_style_state"));
    assert!(checkbox_src.contains("binary_style_state"));
    assert!(toggle_src.contains("binary_style_state"));
}
