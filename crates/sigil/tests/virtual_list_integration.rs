#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic, clippy::trivially_copy_pass_by_ref, clippy::unwrap_used)]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use sigil::hal::{KeyEventInit, MouseButton, MouseButtons, PhysicalKey, Point, PointerEventInit};
use sigil::prelude::*;
use sigil::widgets::{SelectionMode, virtual_list};

const ROW_HEIGHT: f32 = 32.0;
const VIEWPORT_WIDTH: f32 = 1_280.0;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
    }
}

#[test]
fn widget_renders_only_visible_window_at_100k_items() {
    sigil::reactive::Runtime::scope("sigil-virtual-list-integration-window", || {
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..100_000).collect();
        let root = cx.add(
            virtual_list(items, row)
                .row_height(ROW_HEIGHT)
                .viewport_height(480.0)
                .viewport_width(VIEWPORT_WIDTH)
                .scroll_bind(signal(0.0_f32)),
        );

        assert!(count_role(&cx, root, Role::ListItem) <= 45);
    });
}

#[test]
fn scroll_signal_drives_window_advance() {
    sigil::reactive::Runtime::scope("sigil-virtual-list-integration-scroll", || {
        let scroll_y = signal(0.0_f32);
        let rendered = Rc::new(RefCell::new(Vec::new()));
        build_recording_list(scroll_y, Rc::clone(&rendered));
        assert_eq!(rendered.borrow().first().copied(), Some(0));

        scroll_y.set(32_000.0);
        build_recording_list(scroll_y, Rc::clone(&rendered));
        let first = rendered
            .borrow()
            .first()
            .copied()
            .expect("visible row materialised");
        assert_eq!(first, 985, "first row was {first}");
    });
}

#[test]
fn keyboard_navigation_end_to_end() {
    sigil::reactive::Runtime::scope("sigil-virtual-list-integration-keyboard", || {
        let focused_row = signal(0_usize);
        let scroll_y = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let items: Vec<usize> = (0..100).collect();
        let root = cx.add(
            virtual_list(items, row)
                .row_height(ROW_HEIGHT)
                .viewport_height(96.0)
                .viewport_width(320.0)
                .keyboard(true)
                .scroll_bind(scroll_y)
                .focused_row_bind(focused_row),
        );
        let layout = compute_layout(&mut cx, root, Size::fixed(320.0, 96.0));
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        press_down(&mut router, &mut cx, &layout);
        assert_eq!(focused_row.get(), 1);

        for _ in 0..10 {
            press_down(&mut router, &mut cx, &layout);
        }
        assert_eq!(focused_row.get(), 11);
        assert!(scroll_y.get() > 0.0);
    });
}

#[test]
fn selection_propagates_to_a11y() {
    sigil::reactive::Runtime::scope("sigil-virtual-list-integration-selection", || {
        let mut builder = virtual_list((0..20).collect::<Vec<_>>(), row)
            .row_height(ROW_HEIGHT)
            .viewport_height(480.0)
            .viewport_width(320.0)
            .selection(SelectionMode::Single);
        let selection = builder.selection_state();
        selection.update(|state| state.click(5));

        let mut cx = Cx::new_root();
        let root = cx.add(builder);
        let layout = compute_layout(&mut cx, root, Size::fixed(320.0, 480.0));
        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            pointer_down(1.0, ROW_HEIGHT.mul_add(5.0, 1.0)),
        );

        let selected = selected_a11y_count(&cx, root);
        assert_eq!(selected, 1);
    });
}

#[test]
fn scroll_step_meets_144hz_budget() {
    sigil::reactive::Runtime::scope("sigil-virtual-list-integration-budget", || {
        let scroll_y = signal(0.0_f32);
        let items: Vec<usize> = (0..100_000).collect();
        let mut samples = Vec::new();
        for n in 0..15 {
            scroll_y.update(|value| *value += ROW_HEIGHT);
            let frame_items = items.clone();
            let start = Instant::now();
            let mut cx = Cx::new_root();
            let root = cx.add(
                virtual_list(frame_items, row)
                    .row_height(ROW_HEIGHT)
                    .viewport_height(VIEWPORT_WIDTH)
                    .viewport_width(VIEWPORT_WIDTH)
                    .scroll_bind(scroll_y),
            );
            let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
            engine
                .compute(&mut cx, root, Size::fixed(VIEWPORT_WIDTH, VIEWPORT_WIDTH))
                .expect("layout computes");
            if n >= 5 {
                samples.push(start.elapsed());
            }
        }
        samples.sort_unstable();
        let median = samples.get(samples.len() / 2).copied().expect("10 samples");
        let median_ms = median.as_secs_f64() * 1_000.0;
        eprintln!("virtual_list scroll_step median: {median_ms:.3} ms");
        assert!(
            median < Duration::from_secs_f64(0.006_94),
            "median per-frame time {median_ms:.3} ms exceeds 6.94 ms"
        );
    });
}

fn build_recording_list(scroll_y: Signal<f32>, rendered: Rc<RefCell<Vec<usize>>>) {
    rendered.borrow_mut().clear();
    let mut cx = Cx::new_root();
    let items: Vec<usize> = (0..2_000).collect();
    let _ = cx.add(
        virtual_list(items, move |index, item| {
            rendered.borrow_mut().push(index);
            row(index, item)
        })
        .row_height(ROW_HEIGHT)
        .viewport_height(480.0)
        .viewport_width(320.0)
        .scroll_bind(scroll_y),
    );
}

fn row(index: usize, _item: &usize) -> NodeBuilder {
    container()
        .role(Role::ListItem)
        .label(format!("Row {index}"))
        .layout(LayoutStyle::flex_row().fixed_size(320.0, ROW_HEIGHT))
        .child(container().layout(LayoutStyle::block().fixed_size(80.0, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(160.0, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(80.0, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(0.0, ROW_HEIGHT)))
}

fn count_role(cx: &Cx, root: NodeId, role: Role) -> usize {
    let mut count = 0;
    cx.visit_preorder(root, |_, node| {
        if node.a11y.role == role {
            count += 1;
        }
    });
    count
}

fn selected_a11y_count(cx: &Cx, root: NodeId) -> usize {
    let mut count = 0;
    cx.visit_preorder(root, |_, node| {
        if node.a11y.selected.is_some_and(|selected| selected.peek()) {
            count += 1;
        }
    });
    count
}

fn compute_layout(cx: &mut Cx, root: NodeId, size: Size) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout.compute(cx, root, size).expect("layout computes");
    layout
}

fn press_down(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine) {
    router.route(
        cx,
        layout,
        ShellEvent::Key(KeyEvent::new(KeyEventInit {
            key: Key::ArrowDown.into(),
            physical: PhysicalKey::ArrowDown,
            modifiers: ModifiersState::empty(),
            repeat: false,
            phase: KeyPhase::Down,
            text: None,
        })),
    );
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
