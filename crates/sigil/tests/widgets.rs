#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use crate::widget_support;

use sigil::events::PhysicalKey;
use sigil::kernel::{Cx, NodeId};
use sigil_kittest::FakeMeasure;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::events::Event;
use sigil::geometry::Point;
use sigil::layout::set_text_geometry;
use sigil::prelude::*;
use sigil::scene::{SceneCommand, emit_scene};

struct CutMemory(RefCell<Option<String>>);

impl sigil::clipboard::Clipboard for CutMemory {
    fn set(&self, text: &str) -> Result<sigil::clipboard::Delivery, sigil::Error> {
        *self.0.borrow_mut() = Some(text.to_owned());
        Ok(sigil::clipboard::Delivery::Confirmed)
    }
    fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}
use widget_support::{assert_approx, rect_colors};

const TEXT_MEASURE: FakeMeasure = FakeMeasure::chars(7.0, 16.0);

#[path = "widgets/binary.rs"]
mod binary;
#[path = "widgets/builder_hooks.rs"]
mod builder_hooks;
#[path = "widgets/button.rs"]
mod button;
#[path = "widgets/slider.rs"]
mod slider;
#[path = "widgets/text_edit.rs"]
mod text_edit;
#[path = "widgets/text_field.rs"]
mod text_field;

fn ctrl_shift_key(ch: &str, physical: PhysicalKey) -> Event {
    Event::key(Key::character(ch), KeyPhase::Down)
        .physical(physical)
        .modifiers(Modifiers::CTRL | Modifiers::SHIFT)
}

fn type_char_owned(ch: String, physical: PhysicalKey) -> Event {
    Event::key(Key::character(ch.as_str()), KeyPhase::Down)
        .physical(physical)
        .text(Some(ch))
}

fn first_rect_x_for_color(cx: &Cx, root: NodeId, color: sigil::style::Color) -> f32 {
    emit_scene(cx, root, None)
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::Rect(rect) if rect.color == color => Some(rect.rect.origin.x),
            _ => None,
        })
        .expect("rect with requested color exists")
}

fn scene_has_text(cx: &Cx, root: NodeId, expected: &str) -> bool {
    emit_scene(cx, root, None).find_text(expected).is_some()
}

const fn pointer_down(x: f32, y: f32) -> Event {
    pointer_event(
        x,
        y,
        PointerPhase::Down,
        Some(MouseButton::Left),
        MouseButtons::LEFT,
    )
}

const fn pointer_move(x: f32, y: f32) -> Event {
    pointer_event(x, y, PointerPhase::Move, None, MouseButtons::LEFT)
}

const fn pointer_up(x: f32, y: f32) -> Event {
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
) -> Event {
    Event::pointer(phase, Point::new(x, y))
        .button(button)
        .buttons(buttons)
}

fn text_down(text: &str) -> Event {
    Event::key(Key::character(text), KeyPhase::Down)
        .physical(PhysicalKey::KeyA)
        .text(Some(text.into()))
}

fn backspace_down() -> Event {
    Event::key(NamedKey::Backspace, KeyPhase::Down).physical(PhysicalKey::Backspace)
}

fn enter_down() -> Event {
    Event::key(NamedKey::Enter, KeyPhase::Down).physical(PhysicalKey::Enter)
}

fn space_down() -> Event {
    Event::key(Key::character(" "), KeyPhase::Down).physical(PhysicalKey::Space)
}

fn arrow_down(key: NamedKey, physical: PhysicalKey) -> Event {
    Event::key(key, KeyPhase::Down).physical(physical)
}

const SLIDER_TRACK: f32 = 180.0;

fn named_key(key: NamedKey, physical: PhysicalKey) -> Event {
    Event::key(key, KeyPhase::Down).physical(physical)
}

fn shift_arrow(key: NamedKey, physical: PhysicalKey) -> Event {
    Event::key(key, KeyPhase::Down)
        .physical(physical)
        .modifiers(Modifiers::SHIFT)
}

fn ctrl_key(ch: &str, physical: PhysicalKey) -> Event {
    Event::key(Key::character(ch), KeyPhase::Down)
        .physical(physical)
        .modifiers(Modifiers::CTRL)
}
