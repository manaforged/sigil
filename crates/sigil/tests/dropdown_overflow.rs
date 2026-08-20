//! A dropdown with more options than the popup shows must clip and scroll
//! them, not paint the overflow past the popup onto sibling UI.
//!
//! Before this, the popup box was capped at `POPUP_MAX_VISIBLE` rows but all
//! options were appended as plain children with no clip and no scroll, so
//! options 7+ painted below the popup and were unreachable by keyboard
//! (the cursor clamps to n-1). Verified against the reviewer's review.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{KeyEvent, KeyEventInit, KeyPhase, PhysicalKey};

/// The popup box height for `POPUP_MAX_VISIBLE` (6) rows of 36px.
// Mirrors popup.rs: POPUP_MAX_VISIBLE * OPTION_HEIGHT (6 * 28).
const POPUP_VISIBLE_H: f32 = 6.0 * 28.0;

fn has_scrollview(cx: &Cx, id: NodeId) -> bool {
    let Some(node) = cx.get(id) else { return false };
    if node.a11y.role == Role::ScrollView {
        return true;
    }
    node.children().iter().any(|c| has_scrollview(cx, *c))
}

/// The tallest laid-out box under `id` whose role is `ScrollView`, as
/// (viewport height, content extent). None when there is no scroll view.
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

        // Closed dropdowns do not keep an invisible option tree alive.
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
        // The paint-time scissor is what actually stops options painting
        // past the popup onto sibling UI. `clips()` is the layout's promise
        // that the renderer will scissor this box.
        assert!(
            layout.result(sv).unwrap().clips(),
            "the scroll viewport must clip so overflow options do not paint \
             past the popup",
        );
    });
}
