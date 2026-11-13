use super::*;
use sigil::events::EventRouter;
use sigil::geometry::Constraint;
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use sigil_kittest::Harness;

#[test]
fn checkbox_click_flips_signal() {
    sigil::reactive::Owner::scope("sigil-widget-checkbox-click", || {
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
        let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Constraint::fixed(320.0, 120.0))
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
    sigil::reactive::Owner::scope("sigil-widget-checkbox-keyboard", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).label("Monitor"));
        let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Constraint::fixed(320.0, 120.0))
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
    sigil::reactive::Owner::scope("sigil-widget-checkbox-disabled", || {
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
        let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Constraint::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        router.route(&mut cx, &layout, pointer_up(5.0, 5.0));

        assert!(!checked.get());
        assert_eq!(changes.get(), 0);
        assert!(!router.is_focus_target(&cx, &layout, root));
        assert!(!router.is_click_target(&cx, &layout, root));
    });
}

#[test]
fn checkbox_scene_emits_checked_and_unchecked_variants() {
    sigil::reactive::Owner::scope("sigil-widget-checkbox-scene", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(checkbox(checked).label("Visual").tone(Tone::Primary));
        let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Constraint::fixed(320.0, 120.0))
            .expect("layout computes");

        let theme = Theme::default();
        let unchecked_colors = rect_colors(&cx, root);
        assert!(unchecked_colors.contains(&theme.scale(Tone::Neutral).step(1)));

        checked.set(true);
        let checked_colors = rect_colors(&cx, root);
        assert!(checked_colors.contains(&theme.color(Token::Accent)));
        assert!(
            emit_scene(&cx, root, None)
                .commands
                .iter()
                .any(|command| matches!(command, SceneCommand::Path(_))),
            "checked scene must draw the vector check mark"
        );
    });
}

#[test]
fn checkbox_paints_a_light_mark_only_when_checked() {
    assert_eq!(mark_pixels(false), 0, "unchecked box has no mark");
    let lit = mark_pixels(true);
    assert!(lit >= 8, "checked box must paint the mark, got {lit} px");
}

const BOX_CORNER_PX: u32 = 8;

fn mark_pixels(checked: bool) -> usize {
    let mut ui = Harness::new(move || checkbox(signal(checked)).tone(Tone::Primary))
        .pixels(64, 64)
        .scale(2.0);
    let frame = match ui.image() {
        Ok(Some(frame)) => frame,
        other => panic!("cpu frame: {:?}", other.map(|frame| frame.is_some())),
    };
    let (width, height) = frame.dimensions();
    let pixels = || (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
    let accent: Vec<(u32, u32)> = pixels()
        .filter(|&(x, y)| {
            let [r, _, b, _] = frame.get_pixel(x, y).0;
            u16::from(b) > u16::from(r) + 40
        })
        .collect();
    let span = |at: fn(&(u32, u32)) -> u32| {
        let low = accent.iter().map(at).min().unwrap_or(0);
        let high = accent.iter().map(at).max().unwrap_or(0);
        low + BOX_CORNER_PX..high.saturating_sub(BOX_CORNER_PX)
    };
    let (across, down) = (span(|at| at.0), span(|at| at.1));
    pixels()
        .filter(|(x, y)| across.contains(x) && down.contains(y))
        .filter(|&(x, y)| frame.get_pixel(x, y).0[..3].iter().all(|c| *c > 220))
        .count()
}

#[test]
fn a_checkbox_with_a_signal_label_is_named_by_that_label() {
    sigil::reactive::Owner::scope("sigil-widget-checkbox-signal-label", || {
        let label = signal(String::from("Restock alerts"));
        let mut ui = Harness::new(move || checkbox(signal(false)).label(label));
        assert_eq!(
            ui.get((Role::CheckBox, "Restock alerts")).name(),
            Some("Restock alerts")
        );

        label.set(String::from("Price alerts"));
        ui.settle();
        assert_eq!(
            ui.get((Role::CheckBox, "Price alerts")).name(),
            Some("Price alerts")
        );
    });
}

#[test]
fn a_checkbox_with_a_closure_label_is_renamed_with_it() {
    sigil::reactive::Owner::scope("sigil-widget-checkbox-closure-label", || {
        let label = signal(String::from("Restock alerts"));
        let mut ui = Harness::new(move || checkbox(signal(false)).label(move || label.get()));
        assert_eq!(
            ui.get((Role::CheckBox, "Restock alerts")).name(),
            Some("Restock alerts")
        );

        label.set(String::from("Price alerts"));
        ui.settle();
        assert_eq!(
            ui.get((Role::CheckBox, "Price alerts")).name(),
            Some("Price alerts")
        );
    });
}

#[test]
fn toggle_click_flips_signal() {
    sigil::reactive::Owner::scope("sigil-widget-toggle-click", || {
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
        let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Constraint::fixed(320.0, 120.0))
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
    sigil::reactive::Owner::scope("sigil-widget-toggle-keyboard", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).label("Monitor"));
        let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Constraint::fixed(320.0, 120.0))
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
    sigil::reactive::Owner::scope("sigil-widget-toggle-scene", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(toggle(on).label("Visual"));
        let mut layout = LayoutEngine::new(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Constraint::fixed(320.0, 120.0))
            .expect("layout computes");

        let theme = Theme::default();
        let off_colors = rect_colors(&cx, root);
        let off_thumb_x = first_rect_x_for_color(&cx, root, theme.color(Tone::Neutral.step(11)));
        assert!(off_colors.contains(&theme.color(Tone::Neutral.step(6))));

        on.set(true);
        layout
            .compute(&mut cx, root, Constraint::fixed(320.0, 120.0))
            .expect("layout recomputes");
        for _ in 0..120 {
            sigil::animation::tick(1.0 / 60.0);
        }
        let on_colors = rect_colors(&cx, root);
        let on_thumb_x = first_rect_x_for_color(&cx, root, theme.color(Token::OnAccent));
        assert!(on_colors.contains(&theme.color(Token::Accent)));
        assert!(on_thumb_x > off_thumb_x);
    });
}

#[test]
fn toggle_and_checkbox_share_styled_interaction_path() {
    let interaction_src = include_str!("../../src/widgets/interaction.rs");
    let shared_src = include_str!("../../src/widgets/shared.rs");
    let press_src = include_str!("../../src/widgets/shared/press.rs");
    let checkbox_src = include_str!("../../src/widgets/checkbox.rs");
    let toggle_src = include_str!("../../src/widgets/toggle.rs");

    assert!(interaction_src.contains("fn style_state"));
    assert!(interaction_src.contains("fn release_pointer"));
    assert!(press_src.contains("interaction.release_pointer"));
    assert!(press_src.contains("interaction.activate_key"));
    assert!(shared_src.contains("binary_style_state"));
    assert!(checkbox_src.contains("binary_style_state"));
    assert!(toggle_src.contains("binary_style_state"));
}

fn named_by_its_signal_label<N: IntoNode + 'static>(
    role: Role,
    [first, second]: [&'static str; 2],
    build: impl FnOnce(Signal<String>) -> N + 'static,
) {
    sigil::reactive::Owner::scope("sigil-widget-signal-label", || {
        let label = signal(String::from(first));
        let mut ui = Harness::new(move || build(label));
        assert_eq!(ui.get((role, first)).name(), Some(first));

        label.set(String::from(second));
        ui.settle();
        assert_eq!(ui.get((role, second)).name(), Some(second));
    });
}

#[test]
fn a_gauge_with_a_signal_label_is_named_by_that_label() {
    named_by_its_signal_label(Role::ProgressIndicator, ["Stock", "Orders"], |label| {
        gauge(signal(0.5)).label(label)
    });
}

#[test]
fn a_stat_with_a_signal_label_is_named_by_that_label() {
    named_by_its_signal_label(Role::Group, ["Stock", "Orders"], |label| stat(label, "42"));
}

#[test]
fn an_empty_state_with_a_signal_title_is_named_by_that_title() {
    named_by_its_signal_label(Role::Status, ["No stock", "No orders"], |title| {
        empty_state(title, "Nothing here yet")
    });
}

#[test]
fn an_avatar_with_signal_initials_is_named_by_them() {
    named_by_its_signal_label(Role::Image, ["TG", "MK"], avatar);
}
