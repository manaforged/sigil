use serde_json::{Value, json};

use crate::events::{
    KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, MouseButton, MouseButtons,
    PointerEvent, PointerEventInit, PointerKind, PointerPhase, ScrollDelta, ScrollEvent,
    ScrollEventInit, ScrollPhase, ShellEvent,
};
use crate::geometry::Point;

pub(super) fn events_of(request: &Value, unit: f32) -> Result<Vec<ShellEvent>, String> {
    match request.get("op").and_then(Value::as_str) {
        Some(op @ ("click" | "move" | "down" | "up")) => pointer_ops(request, op, unit),
        Some("scroll") => scroll_op(request, unit),
        Some("key") => Ok([
            key_event(request, KeyPhase::Down),
            key_event(request, KeyPhase::Up),
        ]
        .into_iter()
        .flatten()
        .collect()),
        Some("type") => type_ops(request),
        Some(other) => Err(format!("unknown op {other}")),
        None => Err("missing op".into()),
    }
}

fn num(request: &Value, key: &str) -> Option<f64> {
    request.get(key)?.as_f64()
}

fn pos(request: &Value, unit: f32) -> Option<Point> {
    let (x, y) = if let (Some(col), Some(row)) = (num(request, "col"), num(request, "row")) {
        (col * f64::from(unit), row * f64::from(unit))
    } else {
        (num(request, "x")?, num(request, "y")?)
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "pointer coordinates are within display range"
    )]
    Some(Point::new(x as f32, y as f32))
}

fn button(request: &Value, phase: PointerPhase) -> Option<MouseButton> {
    if let Some(name) = request.get("button").and_then(Value::as_str) {
        return match name {
            "left" => Some(MouseButton::Left),
            "right" => Some(MouseButton::Right),
            "middle" => Some(MouseButton::Middle),
            "back" => Some(MouseButton::Back),
            "forward" => Some(MouseButton::Forward),
            _ => None,
        };
    }
    if matches!(phase, PointerPhase::Down | PointerPhase::Up) {
        Some(MouseButton::Left)
    } else {
        None
    }
}

fn modifiers(request: &Value) -> ModifiersState {
    let mut mods = ModifiersState::empty();
    for (key, flag) in [
        ("ctrl", ModifiersState::CONTROL),
        ("shift", ModifiersState::SHIFT),
        ("alt", ModifiersState::ALT),
        ("meta", ModifiersState::META),
    ] {
        if request.get(key).and_then(Value::as_bool) == Some(true) {
            mods |= flag;
        }
    }
    mods
}

const fn held(button: Option<MouseButton>, phase: PointerPhase) -> MouseButtons {
    if matches!(phase, PointerPhase::Up | PointerPhase::Leave) {
        return MouseButtons::empty();
    }
    match button {
        Some(MouseButton::Left) => MouseButtons::LEFT,
        Some(MouseButton::Right) => MouseButtons::RIGHT,
        Some(MouseButton::Middle) => MouseButtons::MIDDLE,
        Some(MouseButton::Back) => MouseButtons::BACK,
        Some(MouseButton::Forward) => MouseButtons::FORWARD,
        Some(MouseButton::Other(_)) | None => MouseButtons::empty(),
    }
}

fn pointer(
    request: &Value,
    phase: PointerPhase,
    unit: f32,
    button: Option<MouseButton>,
) -> Option<ShellEvent> {
    let at = pos(request, unit)?;
    Some(ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: at,
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons: held(button, phase),
        modifiers: modifiers(request),
        phase,
    })))
}

fn key_event(request: &Value, phase: KeyPhase) -> Option<ShellEvent> {
    let name = request.get("key").and_then(Value::as_str)?;
    let (key, text) = if name.chars().count() == 1 {
        (
            LogicalKey::character(name),
            Some(smol_str::SmolStr::from(name)),
        )
    } else {
        let named: keyboard_types::NamedKey = name.parse().ok()?;
        (LogicalKey::named(named), None)
    };
    Some(ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key,
        physical: keyboard_types::Code::Unidentified,
        modifiers: modifiers(request),
        repeat: false,
        phase,
        text,
    })))
}

fn pointer_ops(request: &Value, op: &str, unit: f32) -> Result<Vec<ShellEvent>, String> {
    let phase = match op {
        "click" => {
            let press = button(request, PointerPhase::Down);
            return Ok([
                pointer(request, PointerPhase::Move, unit, None),
                pointer(request, PointerPhase::Down, unit, press),
                pointer(request, PointerPhase::Up, unit, press),
            ]
            .into_iter()
            .flatten()
            .collect());
        }
        "move" => PointerPhase::Move,
        "down" => PointerPhase::Down,
        _ => PointerPhase::Up,
    };
    pointer(request, phase, unit, button(request, phase))
        .map(|e| vec![e])
        .ok_or_else(|| format!("{op} needs col/row or x/y"))
}

fn scroll_op(request: &Value, unit: f32) -> Result<Vec<ShellEvent>, String> {
    let at = pos(request, unit).ok_or("scroll needs col/row or x/y")?;
    #[expect(clippy::cast_possible_truncation, reason = "scroll deltas are small")]
    let delta = ScrollDelta::Lines(Point::new(
        num(request, "dx").unwrap_or(0.0) as f32,
        num(request, "dy").unwrap_or(0.0) as f32,
    ));
    Ok(vec![ShellEvent::Scroll(ScrollEvent::new(
        ScrollEventInit {
            position: at,
            delta,
            modifiers: modifiers(request),
            phase: ScrollPhase::Moved,
        },
    ))])
}

fn type_ops(request: &Value) -> Result<Vec<ShellEvent>, String> {
    let text = request
        .get("text")
        .and_then(Value::as_str)
        .ok_or("type needs text")?;
    Ok(text
        .chars()
        .flat_map(|ch| {
            let req = json!({"key": ch.to_string()});
            [
                key_event(&req, KeyPhase::Down),
                key_event(&req, KeyPhase::Up),
            ]
            .into_iter()
            .flatten()
        })
        .collect())
}
