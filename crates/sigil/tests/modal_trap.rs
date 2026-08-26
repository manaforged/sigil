#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil_hal::{KeyEventInit, MouseButton, MouseButtons, PhysicalKey, Point, PointerEventInit};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout");
    layout
}

fn key_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Down,
    }))
}

fn modal_dialog(cx: &Cx, root: NodeId) -> NodeId {
    cx.nth_child(root, 1).expect("modal dialog child")
}

#[test]
fn modal_width_sizes_the_dialog_not_the_overlay() {
    sigil::reactive::Runtime::scope("sigil-modal-dialog-box", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(640.0, 480.0))
                .child(modal(show).width(360.0).height(280.0).title("Compose")),
        );
        let layout = compute(&mut cx, root);
        let modal_root = cx.nth_child(root, 0).expect("modal root");
        let dialog = modal_dialog(&cx, modal_root);
        let overlay = layout.result(modal_root).expect("overlay").rect();
        let frame = layout.result(dialog).expect("dialog").rect();
        assert!(
            (overlay.width - 640.0).abs() < 0.5,
            "overlay fills the host, got {}",
            overlay.width
        );
        assert!(
            (frame.width - 360.0).abs() < 0.5,
            "width() sizes the dialog, got {}",
            frame.width
        );
        assert!(
            (frame.height - 280.0).abs() < 0.5,
            "height() sizes the dialog, got {}",
            frame.height
        );
    });
}

#[test]
fn modal_open_pushes_focus_scope() {
    sigil::reactive::Runtime::scope("sigil-modal-trap-focus-scope", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(640.0, 480.0))
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(20.0, 20.0)),
                )
                .child(modal(show)),
        );
        let layout = compute(&mut cx, root);
        let modal_root = cx.nth_child(root, 1).expect("modal root");
        let dialog = modal_dialog(&cx, modal_root);

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, key_down(Key::Tab, PhysicalKey::Tab));

        assert_eq!(router.focus(), Some(dialog));

        show.set(false);
        router.route(&mut cx, &layout, key_down(Key::Tab, PhysicalKey::Tab));
        assert!(!show.get());
    });
}

#[test]
fn sheet_open_pushes_the_same_focus_scope_as_modal() {
    sigil::reactive::Runtime::scope("sigil-sheet-trap-focus-scope", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(640.0, 480.0))
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(20.0, 20.0)),
                )
                .child(sheet(show).title("Filters")),
        );
        let layout = compute(&mut cx, root);
        let sheet_root = cx.nth_child(root, 1).expect("sheet root");
        let dialog = modal_dialog(&cx, sheet_root);
        let node = cx.get(dialog).expect("dialog");
        assert_eq!(node.a11y.role, Role::Dialog);
        assert_eq!(
            node.a11y.focus_scope,
            Some(sigil_kernel::FocusScopeKind::Modal)
        );

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, key_down(Key::Tab, PhysicalKey::Tab));
        assert_eq!(router.focus(), Some(dialog));
    });
}

/// A dismissed modal must release its focus scope: after `show` flips
/// false (`Display::None`) and layout recomputes, Tab must reach the
/// focusable OUTSIDE the dialog — not stay trapped in an invisible
/// scope (adversarial review finding, 2026-07-12).
#[test]
fn dismissed_modal_releases_focus_trap() {
    sigil::reactive::Runtime::scope("sigil-modal-trap-release", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(640.0, 480.0))
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(20.0, 20.0)),
                )
                .child(modal(show)),
        );
        let mut layout = compute(&mut cx, root);
        let outside = cx.nth_child(root, 0).expect("outside focusable");
        let modal_root = cx.nth_child(root, 1).expect("modal root");
        let dialog = modal_dialog(&cx, modal_root);

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, key_down(Key::Tab, PhysicalKey::Tab));
        assert_eq!(router.focus(), Some(dialog), "modal traps focus while open");

        show.set(false);
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("recompute after dismiss");
        router.route(&mut cx, &layout, key_down(Key::Tab, PhysicalKey::Tab));
        assert_eq!(
            router.focus(),
            Some(outside),
            "Tab after dismissal must escape the hidden dialog's scope"
        );
    });
}

#[test]
fn escape_closes_modal_when_enabled_default() {
    sigil::reactive::Runtime::scope("sigil-modal-trap-escape", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let root = cx.add(modal(show));
        let layout = compute(&mut cx, root);
        let dialog = modal_dialog(&cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(dialog));
        router.route(&mut cx, &layout, key_down(Key::Escape, PhysicalKey::Escape));

        assert!(!show.get());
    });
}

#[test]
fn backdrop_click_closes_when_opt_in() {
    sigil::reactive::Runtime::scope("sigil-modal-trap-backdrop", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let root = cx.add(
            modal(show)
                .dismiss_on_backdrop(true)
                .child(container().layout(LayoutStyle::block().fixed_size(80.0, 24.0))),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        // Click the centre of the dialog's real laid-out rect — the modal
        // centres the dialog in the viewport, so a hardcoded point rots
        // whenever the layout evolves.
        let dialog = modal_dialog(&cx, root);
        let rect = layout.result(dialog).expect("dialog layout").rect();
        let (mid_x, mid_y) = (
            rect.origin.x + rect.width / 2.0,
            rect.origin.y + rect.height / 2.0,
        );
        assert!(
            !(rect.origin.x <= 5.0 && rect.origin.y <= 5.0),
            "backdrop probe point (5,5) must lie outside the dialog rect {rect:?}"
        );
        router.route(&mut cx, &layout, pointer_down(mid_x, mid_y));
        assert!(show.get(), "dialog click must not dismiss");

        router.route(&mut cx, &layout, pointer_down(5.0, 5.0));
        assert!(!show.get(), "backdrop click dismisses");
    });
}

#[test]
fn three_modal_stack_escape_pops_one_at_a_time() {
    sigil::reactive::Runtime::scope("sigil-modal-trap-stack", || {
        let first = signal(true);
        let second = signal(true);
        let third = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(640.0, 480.0))
                .child(modal(first).title("First"))
                .child(modal(second).title("Second"))
                .child(modal(third).title("Third")),
        );
        let layout = compute(&mut cx, root);
        let top_root = cx.nth_child(root, 2).expect("third modal root");
        let top_dialog = modal_dialog(&cx, top_root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(top_dialog));

        router.route(&mut cx, &layout, key_down(Key::Escape, PhysicalKey::Escape));
        assert_eq!(
            (first.get(), second.get(), third.get()),
            (true, true, false)
        );

        router.route(&mut cx, &layout, key_down(Key::Escape, PhysicalKey::Escape));
        assert_eq!(
            (first.get(), second.get(), third.get()),
            (true, false, false)
        );

        router.route(&mut cx, &layout, key_down(Key::Escape, PhysicalKey::Escape));
        assert_eq!(
            (first.get(), second.get(), third.get()),
            (false, false, false)
        );
    });
}
