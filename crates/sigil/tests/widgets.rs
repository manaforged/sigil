#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::hal::{KeyEventInit, Point, PointerEventInit};
use sigil::layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil::prelude::*;
use sigil::renderer::{SceneCommand, emit_scene};

struct CutMemory(RefCell<Option<String>>);

impl sigil::clipboard::Clipboard for CutMemory {
    fn set(&self, text: &str) -> Result<(), sigil::widgets::ClipError> {
        *self.0.borrow_mut() = Some(text.to_owned());
        Ok(())
    }
    fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}
use sigil::style::TextStyle;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        let len = match content {
            TextContent::Static(s) => s.len(),
            TextContent::Reactive(sig) => sig.peek().len(),
            TextContent::Compute(_) => content.resolve().len(),
        };
        sigil::hal::LogicalSize::new(len as f32 * 7.0, 16.0)
    }
}

struct StubGeometry;

impl TextGeometry for StubGeometry {
    fn hit(
        &self,
        text: &str,
        _style: &TextStyle,
        _width: Option<f32>,
        point: sigil::hal::Point,
    ) -> sigil::hal::HitPosition {
        sigil::hal::HitPosition {
            byte_index: ((point.x / 10.0).round().max(0.0) as usize).min(text.len()),
            affinity: sigil::hal::TextAffinity::Downstream,
        }
    }

    fn caret(
        &self,
        _text: &str,
        _style: &TextStyle,
        _width: Option<f32>,
        position: sigil::hal::HitPosition,
    ) -> CaretGeometry {
        let byte = position.byte_index;
        CaretGeometry {
            x: byte as f32 * 10.0,
            top: 0.0,
            height: 16.0,
        }
    }

    fn selection_rects(
        &self,
        _text: &str,
        _style: &TextStyle,
        _width: Option<f32>,
        _selection: (sigil::hal::HitPosition, sigil::hal::HitPosition),
    ) -> Vec<sigil::hal::Rect> {
        Vec::new()
    }
}

#[path = "widgets/binary.rs"]
mod binary;
#[path = "widgets/button.rs"]
mod button;
#[path = "widgets/slider.rs"]
mod slider;
#[path = "widgets/text_edit.rs"]
mod text_edit;
#[path = "widgets/text_field.rs"]
mod text_field;

fn ctrl_shift_key(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::CONTROL | ModifiersState::SHIFT,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn type_char(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.to_string().into()),
    }))
}

fn type_char_owned(ch: String, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch.as_str()),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.into()),
    }))
}

fn scene_rect_colors(cx: &Cx, root: NodeId) -> Vec<sigil::style::Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(rect) => Some(rect.color),
            _ => None,
        })
        .collect()
}

fn first_rect_x_for_color(cx: &Cx, root: NodeId, color: sigil::style::Rgba8) -> f32 {
    emit_scene(cx, root)
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::Rect(rect) if rect.color == color => Some(rect.rect.origin.x),
            _ => None,
        })
        .expect("rect with requested color exists")
}

fn scene_has_text(cx: &Cx, root: NodeId, expected: &str) -> bool {
    emit_scene(cx, root).find_text(expected).is_some()
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    pointer_event(
        x,
        y,
        PointerPhase::Down,
        Some(MouseButton::Left),
        MouseButtons::LEFT,
    )
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    pointer_event(x, y, PointerPhase::Move, None, MouseButtons::LEFT)
}

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
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
) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

fn text_down(text: &str) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(text),
        physical: PhysicalKey::KeyA,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(text.into()),
    }))
}

fn backspace_down() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Backspace.into(),
        physical: PhysicalKey::Backspace,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn enter_down() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Enter.into(),
        physical: PhysicalKey::Enter,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn space_down() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(" "),
        physical: PhysicalKey::Space,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn arrow_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

const SLIDER_TRACK: f32 = 180.0;

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

fn named_key(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn shift_arrow(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::SHIFT,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn ctrl_key(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}
