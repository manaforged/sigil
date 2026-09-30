use super::*;
use sigil::events::EventRouter;
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;

#[test]
fn text_field_typing_updates_signal() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-typing", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Notes"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, text_down("a"));
        router.route(&mut cx, &layout, text_down("é"));

        assert_eq!(value.get(), "aé");
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::TextInput);
        assert_eq!(node.a11y.text_value.expect("text value").get(), "aé");
        assert_eq!(node.a11y.placeholder.as_deref(), Some("Notes"));
    });
}

#[test]
fn text_field_typing_emits_the_updated_glyph_run() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-typing-paint", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Notes"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, text_down("a"));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout recomputes");

        assert_eq!(value.get(), "a", "typing must update editing state");
        assert!(
            scene_has_text(&cx, root, "a"),
            "typing must replace the placeholder glyph run with the edited value"
        );
    });
}

#[test]
fn text_field_backspace_removes_last_character() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-backspace", || {
        let value = signal(String::from("ab"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, backspace_down());

        assert_eq!(value.get(), "a");
    });
}

#[test]
fn text_field_disabled_ignores_typing() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-disabled", || {
        let value = signal(String::new());
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).disabled(disabled));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, text_down("a"));

        assert_eq!(value.get(), "");
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hittable);
    });
}

#[test]
fn text_field_enter_fires_submit_with_current_value() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-submit", || {
        let value = signal(String::from("ready"));
        let submitted = Rc::new(RefCell::new(String::new()));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).submit({
            let submitted = Rc::clone(&submitted);
            move |text: &str| {
                submitted.replace(text.to_owned());
            }
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, enter_down());

        assert_eq!(submitted.borrow().as_str(), "ready");
        assert_eq!(value.get(), "ready");
    });
}

#[test]
fn text_field_placeholder_renders_when_empty_and_unfocused() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-placeholder", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Add note"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let scene = emit_scene(&cx, root);
        let placeholder = scene
            .commands
            .iter()
            .find_map(|command| match command {
                SceneCommand::GlyphRun(run) if run.run.text == "Add note" => Some(&run.run),
                _ => None,
            })
            .expect("placeholder glyph run");

        assert_eq!(placeholder.color, Theme::default().text_muted());
    });
}

#[test]
fn text_field_placeholder_remains_visible_when_focused() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-placeholder-focus", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Add note"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout recomputes");

        assert!(scene_has_text(&cx, root, "Add note"));
    });
}

#[test]
fn text_field_focus_ring_visible_when_focused() {
    sigil::reactive::Runtime::scope("sigil-widget-text-field-focus-ring", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).placeholder("Notes"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        let node = cx.get(root).unwrap();
        assert!(node.get_extension::<FocusRing>().unwrap().visible());
    });
}
