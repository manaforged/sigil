use super::*;
use crate::kernel::FocusScopeKind;
use crate::kernel::*;
use crate::prelude::button;
use crate::reactive::{Runtime, signal};
use accesskit::Action::{Click, Decrement, Increment};

struct StubMeasure;
impl crate::layout::TextMeasure for StubMeasure {
    fn measure(
        &self,
        _content: &TextContent,
        _space: crate::geometry::Size,
    ) -> crate::geometry::LogicalSize {
        crate::geometry::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, crate::geometry::Size::fixed(200.0, 100.0))
        .expect("layout computes");
    layout
}

fn act(
    cx: &mut Cx,
    router: &mut EventRouter,
    layout: &LayoutEngine,
    actions: &[accesskit::Action],
    target: NodeId,
) {
    let (tx, rx) = crossbeam_channel::unbounded();
    for &action in actions {
        tx.send(accesskit::ActionRequest {
            action,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target.to_accesskit(),
            data: None,
        })
        .unwrap();
    }
    drain_a11y_actions(&rx, cx, router, layout);
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

        act(&mut cx, &mut router, &layout, &[Increment], root);
        assert!((value.get() - 0.75).abs() < f32::EPSILON, "0.5 + 0.25");

        act(&mut cx, &mut router, &layout, &[Increment, Increment], root);
        assert!((value.get() - 1.0).abs() < f32::EPSILON, "clamps at max");

        act(&mut cx, &mut router, &layout, &[Decrement], root);
        assert!((value.get() - 0.75).abs() < f32::EPSILON, "1.0 − 0.25");
    });
}

#[test]
fn increment_rejects_non_focus_target() {
    Runtime::scope("a11y-increment-gated", || {
        let value = signal(0.5_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(container().value(value, 0.0, 1.0).value_step(0.25));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        act(&mut cx, &mut router, &layout, &[Increment], root);
        assert!(
            (value.get() - 0.5).abs() < f32::EPSILON,
            "unfocusable target is untouched"
        );
    });
}

#[test]
fn click_activates_a_button_once() {
    Runtime::scope("a11y-button", || {
        let clicks = signal(0);
        let mut cx = Cx::new_root();
        let root = cx.add(button("Save").activate(move || clicks.update(|value| *value += 1)));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        let projected =
            crate::a11y::node(&cx, root, &|id| layout.result(id).map(|value| value.rect()));
        assert!(projected.supports_action(accesskit::Action::Click));
        act(&mut cx, &mut router, &layout, &[Click], root);
        assert_eq!(clicks.get(), 1);
    });
}

#[test]
fn click_can_remove_its_own_controls() {
    Runtime::scope("a11y-close", || {
        let visible = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(container().children_reactive(move || {
            if visible.get() {
                vec![button("Cancel").activate(move || visible.set(false))]
            } else {
                vec![]
            }
        }));
        let layout = compute(&mut cx, root);
        let control = *cx.get(root).unwrap().children().first().unwrap();
        let mut router = EventRouter::new();
        act(&mut cx, &mut router, &layout, &[Click], control);
        compute(&mut cx, root);
        assert!(!visible.get());
        assert!(cx.get(control).is_none());
    });
}

#[test]
fn click_rechecks_the_focus_scope_after_a_dialog_closes() {
    Runtime::scope("a11y-dialog-reopen", || {
        let visible = signal(true);
        let clicks = signal(0);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .child(button("Edit").activate(move || clicks.update(|value| *value += 1)))
                .child(container().children_reactive(move || {
                    if visible.get() {
                        vec![
                            container()
                                .mutate(|node| {
                                    node.a11y.focus_scope = Some(FocusScopeKind::Modal);
                                })
                                .child(button("Save").activate(move || visible.set(false))),
                        ]
                    } else {
                        vec![]
                    }
                })),
        );
        let layout = compute(&mut cx, root);
        let edit = *cx.get(root).unwrap().children().first().unwrap();
        let mut router = EventRouter::new();
        router.focus_if_needed(&mut cx, &layout);
        act(&mut cx, &mut router, &layout, &[Click], edit);
        assert_eq!(clicks.get(), 0, "the open dialog blocks background actions");
        visible.set(false);
        let reopened = compute(&mut cx, root);
        act(&mut cx, &mut router, &reopened, &[Click], edit);
        assert_eq!(
            clicks.get(),
            1,
            "closing the dialog releases the Edit action"
        );
    });
}
