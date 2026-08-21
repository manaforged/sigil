use super::*;
use sigil_kernel::prelude::*;
use sigil_reactive::{Runtime, signal};

struct StubMeasure;
impl sigil_layout::TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, sigil_hal::Size::fixed(200.0, 100.0))
        .expect("layout computes");
    layout
}

fn action_request(
    action: accesskit::Action,
    target: sigil_kernel::NodeId,
) -> accesskit::ActionRequest {
    accesskit::ActionRequest {
        action,
        target_tree: accesskit::TreeId::ROOT,
        target_node: target.to_accesskit(),
        data: None,
    }
}

#[test]
fn increment_adjusts_value_by_step_and_clamps() {
    Runtime::scope("a11y-increment-step", || {
        let value = signal(0.5_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .focusable(true)
                .value(value, 0.0, 1.0)
                .value_step(0.25),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        let (tx, rx) = crossbeam_channel::unbounded();

        let _ = tx.send(action_request(accesskit::Action::Increment, root));
        drain_a11y_actions(&rx, &mut cx, &mut router, &layout);
        assert!((value.get() - 0.75).abs() < f32::EPSILON, "0.5 + 0.25");

        let _ = tx.send(action_request(accesskit::Action::Increment, root));
        let _ = tx.send(action_request(accesskit::Action::Increment, root));
        drain_a11y_actions(&rx, &mut cx, &mut router, &layout);
        assert!((value.get() - 1.0).abs() < f32::EPSILON, "clamps at max");

        let _ = tx.send(action_request(accesskit::Action::Decrement, root));
        drain_a11y_actions(&rx, &mut cx, &mut router, &layout);
        assert!((value.get() - 0.75).abs() < f32::EPSILON, "1.0 − 0.25");
    });
}

#[test]
fn increment_rejects_non_focus_target() {
    Runtime::scope("a11y-increment-gated", || {
        let value = signal(0.5_f32);
        let mut cx = Cx::new_root();
        // No `.focusable(true)` — the gate must reject the action.
        let root = cx.add(container().value(value, 0.0, 1.0).value_step(0.25));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        let (tx, rx) = crossbeam_channel::unbounded();
        let _ = tx.send(action_request(accesskit::Action::Increment, root));
        drain_a11y_actions(&rx, &mut cx, &mut router, &layout);
        assert!(
            (value.get() - 0.5).abs() < f32::EPSILON,
            "unfocusable target is untouched"
        );
    });
}
