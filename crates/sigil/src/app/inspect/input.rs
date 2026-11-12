use serde_json::{Value, json};

use crate::events::{
    Event, KeyPhase, LogicalKey, Modifiers, MouseButton, MouseButtons, PointerPhase, ScrollDelta,
    ScrollPhase,
};
use crate::geometry::Point;

#[derive(Clone, Copy)]
enum Op {
    Click,
    Pointer(PointerPhase),
    Scroll,
    Key,
    Type,
}

pub(super) fn is_input(request: &Value) -> bool {
    request
        .get("op")
        .and_then(Value::as_str)
        .and_then(parse_op)
        .is_some()
}

fn parse_op(name: &str) -> Option<Op> {
    Some(match name {
        "click" => Op::Click,
        "move" => Op::Pointer(PointerPhase::Move),
        "down" => Op::Pointer(PointerPhase::Down),
        "up" => Op::Pointer(PointerPhase::Up),
        "scroll" => Op::Scroll,
        "key" => Op::Key,
        "type" => Op::Type,
        _ => return None,
    })
}

pub(super) fn events_of(request: &Value, unit: f32) -> Result<Vec<Event>, String> {
    let name = request
        .get("op")
        .and_then(Value::as_str)
        .ok_or("missing op")?;
    match parse_op(name).ok_or_else(|| format!("unknown op {name}"))? {
        Op::Click => Ok(click(request, unit)),
        Op::Pointer(phase) => pointer(request, phase, unit, button(request, phase))
            .map(|event| vec![event])
            .ok_or_else(|| format!("{name} needs col/row or x/y")),
        Op::Scroll => scroll_op(request, unit),
        Op::Key => Ok(key_press(request)),
        Op::Type => type_ops(request),
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

fn modifiers(request: &Value) -> Modifiers {
    let mut mods = Modifiers::empty();
    for (key, flag) in [
        ("ctrl", Modifiers::CTRL),
        ("shift", Modifiers::SHIFT),
        ("alt", Modifiers::ALT),
        ("meta", Modifiers::META),
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
) -> Option<Event> {
    let at = pos(request, unit)?;
    Some(
        Event::pointer(phase, at)
            .button(button)
            .buttons(held(button, phase))
            .modifiers(modifiers(request)),
    )
}

fn key_event(request: &Value, phase: KeyPhase) -> Option<Event> {
    let name = request.get("key").and_then(Value::as_str)?;
    let (key, text) = if name.chars().count() == 1 {
        (LogicalKey::character(name), Some(String::from(name)))
    } else {
        let named: crate::events::Key = name.parse().ok()?;
        (LogicalKey::named(named), None)
    };
    Some(
        Event::key(key, phase)
            .modifiers(modifiers(request))
            .text(text),
    )
}

fn click(request: &Value, unit: f32) -> Vec<Event> {
    let press = button(request, PointerPhase::Down);
    [
        pointer(request, PointerPhase::Move, unit, None),
        pointer(request, PointerPhase::Down, unit, press),
        pointer(request, PointerPhase::Up, unit, press),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn scroll_op(request: &Value, unit: f32) -> Result<Vec<Event>, String> {
    let at = pos(request, unit).ok_or("scroll needs col/row or x/y")?;
    #[expect(clippy::cast_possible_truncation, reason = "scroll deltas are small")]
    let delta = ScrollDelta::Lines(Point::new(
        num(request, "dx").unwrap_or(0.0) as f32,
        num(request, "dy").unwrap_or(0.0) as f32,
    ));
    Ok(vec![
        Event::scroll(at, delta, ScrollPhase::Moved).modifiers(modifiers(request)),
    ])
}

fn type_ops(request: &Value) -> Result<Vec<Event>, String> {
    let text = request
        .get("text")
        .and_then(Value::as_str)
        .ok_or("type needs text")?;
    Ok(text
        .chars()
        .flat_map(|ch| key_press(&json!({"key": ch.to_string()})))
        .collect())
}

fn key_press(request: &Value) -> Vec<Event> {
    [KeyPhase::Down, KeyPhase::Up]
        .into_iter()
        .filter_map(|phase| key_event(request, phase))
        .collect()
}
