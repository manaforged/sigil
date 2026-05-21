use super::*;

use app::DefaultTextMeasure;
use hal::{KeyEventInit, Point, PointerEventInit, Rect};
use renderer::emit_scene;

#[test]
fn slice_emits_initial_and_reactive_text() -> core::result::Result<(), Box<dyn std::error::Error>> {
    sigil::reactive::Runtime::scope("vertical-slice-test", || {
        let mut cx = Cx::new_root();
        let state = DashboardState::new();
        let root = cx.add(app_shell(state));
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));

        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;
        assert_scene_text(&cx, root, "Ready. Pick a module and queue a run.");

        select_module(state, 1);
        state.queue_run();
        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;
        assert_scene_text(&cx, root, "Run 1 queued for Fanatics queue.");
        Ok(())
    })
}

#[test]
fn focused_button_enter_routes_through_event_system()
-> core::result::Result<(), Box<dyn std::error::Error>> {
    sigil::reactive::Runtime::scope("vertical-slice-key-test", || {
        let mut cx = Cx::new_root();
        let root = cx.add(app_shell(DashboardState::new()));
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));
        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;

        let button = find_by_label(&cx, root, "Queue run")
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Queue run button"))?;
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(button));
        router.route(&mut cx, &layout, enter_down());

        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;
        assert_scene_text(&cx, root, "Run 1 queued for Target monitor.");
        Ok(())
    })
}

#[test]
fn slice_checkbox_toggles_status_text() -> core::result::Result<(), Box<dyn std::error::Error>> {
    sigil::reactive::Runtime::scope("vertical-slice-checkbox-test", || {
        let mut cx = Cx::new_root();
        let state = DashboardState::new();
        let root = cx.add(app_shell(state));
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));
        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;

        let checkbox = find_by_label(&cx, root, "Restock alerts").ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Restock alerts checkbox")
        })?;
        let center = center_of(&cx, &layout, checkbox);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(center.x, center.y));
        router.route(&mut cx, &layout, pointer_up(center.x, center.y));

        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;
        assert_scene_text(&cx, root, "Restock alerts enabled.");
        Ok(())
    })
}

#[test]
fn slice_slider_updates_status_text_on_drag() -> core::result::Result<(), Box<dyn std::error::Error>>
{
    sigil::reactive::Runtime::scope("vertical-slice-slider-test", || {
        let mut cx = Cx::new_root();
        let root = cx.add(app_shell(DashboardState::new()));
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));
        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;

        let slider = find_by_role(&cx, root, Role::Slider).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Delay budget slider")
        })?;
        let rect = absolute_rect(&cx, &layout, slider);
        let y = rect.origin.y + (rect.height / 2.0);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(rect.origin.x, y));
        router.route(&mut cx, &layout, pointer_move(rect.origin.x + 135.0, y));

        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;
        assert_scene_text(&cx, root, "Delay budget 75%");
        Ok(())
    })
}

#[test]
fn slice_text_field_updates_status() -> core::result::Result<(), Box<dyn std::error::Error>> {
    sigil::reactive::Runtime::scope("vertical-slice-text-field-status-test", || {
        let mut cx = Cx::new_root();
        let root = cx.add(app_shell(DashboardState::new()));
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));
        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;

        let field = find_by_role(&cx, root, Role::TextInput)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Notes text field"))?;
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(field));
        router.route(&mut cx, &layout, text_down("h"));
        router.route(&mut cx, &layout, text_down("i"));

        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;
        assert_scene_text(&cx, root, "Current note: hi");
        Ok(())
    })
}

#[test]
fn slice_text_field_enter_appends_note() -> core::result::Result<(), Box<dyn std::error::Error>> {
    sigil::reactive::Runtime::scope("vertical-slice-text-field-submit-test", || {
        let mut cx = Cx::new_root();
        let root = cx.add(app_shell(DashboardState::new()));
        let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));
        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;

        let field = find_by_role(&cx, root, Role::TextInput)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Notes text field"))?;
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(field));
        router.route(&mut cx, &layout, text_down("n"));
        router.route(&mut cx, &layout, text_down("o"));
        router.route(&mut cx, &layout, text_down("t"));
        router.route(&mut cx, &layout, text_down("e"));
        router.route(&mut cx, &layout, enter_down());

        layout.compute(&mut cx, root, Size::fixed(960.0, 640.0))?;
        assert_scene_text(&cx, root, "Submitted notes: note");
        Ok(())
    })
}

fn assert_scene_text(cx: &Cx, root: NodeId, expected: &str) {
    let scene = emit_scene(cx, root);
    let found = scene.find_text(expected).is_some();
    assert!(found, "expected scene text: {expected}");
}

fn find_by_label(cx: &Cx, root: NodeId, expected: &str) -> Option<NodeId> {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if found.is_some() {
            return;
        }
        if let AccessibleName::Static(label) = &node.a11y.name
            && label == expected
        {
            found = Some(id);
        }
    });
    found
}

fn find_by_role(cx: &Cx, root: NodeId, expected: Role) -> Option<NodeId> {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if found.is_some() {
            return;
        }
        if node.a11y.role == expected {
            found = Some(id);
        }
    });
    found
}

fn center_of(cx: &Cx, layout: &LayoutEngine, id: NodeId) -> Point {
    let rect = absolute_rect(cx, layout, id);
    Point::new(
        rect.origin.x + (rect.width / 2.0),
        rect.origin.y + (rect.height / 2.0),
    )
}

fn absolute_rect(_cx: &Cx, layout: &LayoutEngine, id: NodeId) -> Rect {
    // `LayoutResult::rect()` is already in root coordinates (see
    // commit a84d207 / hit_test.rs comment): the layout solvers
    // accumulate every ancestor's border + padding offset into the
    // rect they write. Walking parents and re-summing would
    // double-count. Read the rect through unchanged.
    layout.result(id).map_or_else(
        || Rect::new(Point::new(0.0, 0.0), 0.0, 0.0),
        |result| result.rect(),
    )
}

fn enter_down() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Enter.into(),
        physical: PhysicalKey::Enter,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn text_down(text: &str) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(text),
        physical: PhysicalKey::KeyA,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(text.into()),
    }))
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    pointer_event(
        x,
        y,
        PointerPhase::Down,
        Some(MouseButton::Left),
        MouseButtons::LEFT,
    )
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    pointer_event(x, y, PointerPhase::Move, None, MouseButtons::LEFT)
}

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    pointer_event(
        x,
        y,
        PointerPhase::Up,
        Some(MouseButton::Left),
        MouseButtons::empty(),
    )
}

const fn pointer_event(
    x: f32,
    y: f32,
    phase: PointerPhase,
    button: Option<MouseButton>,
    buttons: MouseButtons,
) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}
