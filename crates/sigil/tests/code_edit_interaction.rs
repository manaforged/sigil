#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use sigil::events::{PhysicalKey, ScrollDelta, ScrollPhase};
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil_kittest::FakeMeasure;
use std::cell::Cell;
use std::rc::Rc;

use sigil::events::Event;
use sigil::geometry::Point;
use sigil::prelude::*;

const TEXT_MEASURE: FakeMeasure = FakeMeasure::fixed(1600.0, 16.0);

fn layout_in(cx: &mut Cx, root: NodeId, w: f32, h: f32) -> LayoutEngine {
    let mut engine = LayoutEngine::new(Box::new(TEXT_MEASURE));
    engine.compute(cx, root, Constraint::fixed(w, h)).unwrap();
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

const fn scroll_at(x: f32, y: f32, dy: f32) -> Event {
    Event::scroll(
        Point::new(x, y),
        ScrollDelta::Pixels(Point::new(0.0, dy)),
        ScrollPhase::Moved,
    )
}

fn char_key(text: &str) -> Event {
    Event::key(Key::character(text), KeyPhase::Down)
        .physical(PhysicalKey::KeyX)
        .text(Some(text.into()))
}

fn named_key(key: NamedKey, modifiers: Modifiers) -> Event {
    Event::key(Key::Named(key), KeyPhase::Down)
        .physical(PhysicalKey::KeyX)
        .modifiers(modifiers)
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
