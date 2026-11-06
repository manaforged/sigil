use serde_json::Value;

use super::super::scales::Dot;
use super::Row;
use super::channel::Channel;

const MAX_ROWS: usize = 5_000;
const TOP_KEYS: [&str; 12] = [
    "$schema",
    "data",
    "mark",
    "encoding",
    "title",
    "width",
    "height",
    "description",
    "name",
    "config",
    "autosize",
    "background",
];
const CHANNELS: [&str; 6] = ["x", "y", "color", "theta", "tooltip", "xOffset"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Mark {
    Bar,
    Line,
    Area,
    Point,
    Arc,
}

pub(super) struct Spec {
    pub(super) mark: Mark,
    pub(super) donut: bool,
    pub(super) dot: Dot,
    pub(super) rows: Vec<Row>,
    pub(super) x: Option<Channel>,
    pub(super) y: Option<Channel>,
    pub(super) color: Option<Channel>,
    pub(super) theta: Option<Channel>,
    pub(super) offset: bool,
    pub(super) zero: Option<bool>,
    pub(super) title: Option<String>,
    pub(super) width: Option<f32>,
    pub(super) height: Option<f32>,
}

struct Shape {
    mark: Mark,
    donut: bool,
    dot: Dot,
}

impl Shape {
    fn kind(name: &str, filled: Option<bool>) -> Option<(Mark, Dot)> {
        let hollow = |default: bool| {
            if filled.unwrap_or(default) {
                Dot::Disc
            } else {
                Dot::Ring
            }
        };
        Some(match name {
            "bar" => (Mark::Bar, Dot::Disc),
            "line" => (Mark::Line, Dot::Disc),
            "area" => (Mark::Area, Dot::Disc),
            "point" => (Mark::Point, hollow(false)),
            "circle" => (Mark::Point, hollow(true)),
            "square" => (Mark::Point, Dot::Square),
            "arc" => (Mark::Arc, Dot::Disc),
            _ => return None,
        })
    }

    fn parse(value: &Value) -> Option<Self> {
        let (name, map) = match value {
            Value::String(name) => (name.as_str(), None),
            Value::Object(map) => (map.get("type")?.as_str()?, Some(map)),
            _ => return None,
        };
        let number = |key: &str| map.and_then(|map| map.get(key)).and_then(Value::as_f64);
        let filled = map.and_then(|map| map.get("filled")).and_then(Value::as_bool);
        let (mark, dot) = Self::kind(name, filled)?;
        let donut = number("innerRadius").is_some_and(|radius| radius > 0.0);
        Some(Self { mark, donut, dot })
    }
}

#[allow(
    clippy::option_option,
    reason = "the outer None rejects the spec and the inner None means the key is absent"
)]
pub(super) fn optional<T>(
    value: Option<&Value>,
    parse: impl FnOnce(&Value) -> Option<T>,
) -> Option<Option<T>> {
    value.map_or(Some(None), |value| parse(value).map(Some))
}

fn rows(data: &Value) -> Option<Vec<Row>> {
    let map = data.as_object()?;
    if map.keys().any(|key| key != "values" && key != "name") {
        return None;
    }
    let values = map.get("values")?.as_array()?;
    if values.len() > MAX_ROWS {
        return None;
    }
    values.iter().map(|row| row.as_object().cloned()).collect()
}

fn title(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => title(map.get("text")?),
        Value::Array(lines) => lines
            .iter()
            .map(|line| line.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .map(|lines| lines.join(" ")),
        _ => None,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a chart size is a small pixel count"
)]
fn size(value: Option<&Value>) -> Option<f32> {
    value
        .and_then(Value::as_f64)
        .filter(|px| px.is_finite() && *px > 0.0)
        .map(|px| px as f32)
}

fn zero(config: Option<&Value>) -> Option<bool> {
    config?.get("scale")?.get("zero")?.as_bool()
}

impl Spec {
    pub(super) fn parse(value: &Value) -> Option<Self> {
        let top = value.as_object()?;
        if top.keys().any(|key| !TOP_KEYS.contains(&key.as_str())) {
            return None;
        }
        let shape = Shape::parse(top.get("mark")?)?;
        let encoding = top.get("encoding")?.as_object()?;
        if encoding.keys().any(|key| !CHANNELS.contains(&key.as_str())) {
            return None;
        }
        let channel = |name: &'static str| {
            optional(encoding.get(name), |value| Channel::parse(name, value))
        };
        Some(Self {
            mark: shape.mark,
            donut: shape.donut,
            dot: shape.dot,
            rows: rows(top.get("data")?)?,
            x: channel("x")?,
            y: channel("y")?,
            color: channel("color")?,
            theta: channel("theta")?,
            offset: encoding.contains_key("xOffset"),
            zero: zero(top.get("config")),
            title: optional(top.get("title"), title)?,
            width: size(top.get("width")),
            height: size(top.get("height")),
        })
    }
}
