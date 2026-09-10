#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil_hal::{KeyEvent, KeyEventInit, KeyPhase, PhysicalKey};

const POPUP_VISIBLE_H: f32 = 6.0 * 28.0;

fn has_scrollview(cx: &Cx, id: NodeId) -> bool {
    let Some(node) = cx.get(id) else { return false };
    if node.a11y.role == Role::ScrollView {
        return true;
    }
    node.children().iter().any(|c| has_scrollview(cx, *c))
}

fn scrollview_id(cx: &Cx, id: NodeId) -> Option<NodeId> {
    let node = cx.get(id)?;
    if node.a11y.role == Role::ScrollView {
        return Some(id);
    }
    node.children().iter().find_map(|c| scrollview_id(cx, *c))
}

fn labels(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("Option {i}")).collect()
}

fn open_popup(cx: &mut Cx, root: NodeId, layout: &mut LayoutEngine) {
    let mut router = EventRouter::new();
    router.set_focus(cx, Some(root));
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
    layout
        .compute(cx, root, Size::fixed(640.0, 640.0))
        .expect("relayout after open");
}

#[test]
fn few_options_use_a_plain_popup() {
    sigil::reactive::Runtime::scope("dd-few", || {
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(signal(0_usize), labels(3)).width(200.0));
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil_app::DefaultTextMeasure::new()));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 640.0))
            .expect("layout");
        open_popup(&mut cx, root, &mut layout);
        assert!(
            !has_scrollview(&cx, root),
            "a 3-option dropdown fits the popup and must not add a scroll view",
        );
    });
}

#[test]
fn many_options_scroll_within_the_popup() {
    sigil::reactive::Runtime::scope("dd-many", || {
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(signal(0_usize), labels(20)).width(200.0));
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil_app::DefaultTextMeasure::new()));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 640.0))
            .expect("layout");

        assert!(!has_scrollview(&cx, root));
        open_popup(&mut cx, root, &mut layout);
        assert!(
            has_scrollview(&cx, root),
            "a 20-option dropdown must wrap its options in a scroll view",
        );
        let sv = scrollview_id(&cx, root).expect("open popup has a scroll view");
        let viewport = layout.result(sv).unwrap().rect().height;
        let content = *cx.get(sv).unwrap().children().first().unwrap();
        let extent = layout.result(content).unwrap().rect().height;

        assert!(
            (viewport - POPUP_VISIBLE_H).abs() < 1.0,
            "the scroll viewport should show POPUP_MAX_VISIBLE rows ({POPUP_VISIBLE_H}px), got {viewport}",
        );
        assert!(
            extent > viewport + 1.0,
            "20 options ({extent}px) must exceed the {viewport}px viewport so the \
             overflow is scrollable rather than painted past the popup",
        );
        assert!(
            layout.result(sv).unwrap().clips(),
            "the scroll viewport must clip so overflow options do not paint \
             past the popup",
        );
    });
}
