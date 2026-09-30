#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

#[path = "events_support/input.rs"]
mod input;

use sigil::events::{PhysicalKey, ScrollDelta, ScrollEvent, ScrollPhase};
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil_kittest::FixedMeasure;
use std::cell::Cell;
use std::rc::Rc;

use input::pointer_down;
use sigil::events::ShellEvent;
use sigil::events::{KeyEventInit, ScrollEventInit};
use sigil::geometry::Point;
use sigil::prelude::*;

const TEXT_MEASURE: FixedMeasure = FixedMeasure {
    width: 1600.0,
    height: 16.0,
};

fn layout_in(cx: &mut Cx, root: NodeId, w: f32, h: f32) -> LayoutEngine {
    let mut engine = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
    engine.compute(cx, root, Size::fixed(w, h)).unwrap();
    engine
}

fn first_text_input(cx: &Cx, root: NodeId) -> NodeId {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if found.is_none() && node.a11y.role == Role::TextInput {
            found = Some(id);
        }
    });
    found.expect("a code_edit node with Role::TextInput")
}

const fn scroll_at(x: f32, y: f32, dy: f32) -> ShellEvent {
    ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
        position: Point::new(x, y),
        delta: ScrollDelta::Pixels(Point::new(0.0, dy)),
        modifiers: ModifiersState::empty(),
        phase: ScrollPhase::Moved,
    }))
}

fn char_key(text: &str) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(text),
        physical: PhysicalKey::KeyX,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(text.into()),
    }))
}

fn named_key(key: Key, modifiers: ModifiersState) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::Named(key),
        physical: PhysicalKey::KeyX,
        modifiers,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn caret_bar_present(cx: &Cx, layout: &LayoutEngine, root: NodeId) -> bool {
    let mut found = false;
    cx.visit_preorder(root, |id, _node| {
        if let Some(result) = layout.result(id)
            && (result.rect().height - 14.4).abs() < 0.5
        {
            found = true;
        }
    });
    found
}

#[path = "code_edit_interaction/keys.rs"]
mod keys;
#[path = "code_edit_interaction/scroll.rs"]
mod scroll;
