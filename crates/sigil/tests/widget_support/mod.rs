#![allow(dead_code)]

use sigil::a11y::node as access_node;
use sigil::events::{Event, EventRouter, Key, KeyPhase, PhysicalKey};
use sigil::geometry::Size;
use sigil::kernel::{Cx, NodeId, NodeType, TextContent};
use sigil::layout::{LayoutEngine, TextMeasure};
use sigil::reactive::Signal;
use sigil::scene::{SceneCommand, emit_scene};
use sigil::style::Rgba8;

pub(crate) fn compute(
    cx: &mut Cx,
    root: NodeId,
    viewport: Size,
    measure: Box<dyn TextMeasure>,
) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(measure);
    layout.compute(cx, root, viewport).expect("layout computes");
    layout
}

pub(crate) fn rect_colors(cx: &Cx, root: NodeId) -> Vec<Rgba8> {
    emit_scene(cx, root, None)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r) => Some(r.color),
            _ => None,
        })
        .collect()
}

pub(crate) fn find_text_node(cx: &Cx, root: NodeId) -> Option<NodeId> {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if found.is_none()
            && matches!(
                node.kind,
                NodeType::Text {
                    content: TextContent::Reactive(_),
                    ..
                }
            )
        {
            found = Some(id);
        }
    });
    found
}

pub(crate) fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

#[track_caller]
pub(crate) fn assert_focus_follows_disabled(
    cx: &mut Cx,
    layout: &LayoutEngine,
    root: NodeId,
    disabled: Signal<bool>,
) {
    for off in [true, false, true] {
        disabled.set(off);
        let mut router = EventRouter::new();
        let tab = Event::key(Key::Tab, KeyPhase::Down).physical(PhysicalKey::Tab);
        router.route(cx, layout, tab);
        assert_eq!(router.focus().is_some(), !off, "Tab with disabled={off}");
        let props = &cx.get(root).expect("control").a11y;
        assert_eq!(props.is_focusable(), !off, "a11y with disabled={off}");
        let access = access_node(cx, root, &|_| None);
        assert_eq!(
            access.supports_action(accesskit::Action::Focus),
            !off,
            "accesskit focus action with disabled={off}"
        );
        assert_eq!(access.is_disabled(), off, "accesskit disabled={off}");
    }
}
