use serde_json::Value;

use super::Row;

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
const REJECTED: [&str; 6] = ["bin", "timeUnit", "value", "datum", "condition", "impute"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Mark {
    Bar,
    Line,
    Area,
    Point,
    Arc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    Quantitative,
    Nominal,
    Ordinal,
    Temporal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Aggregate {
    Sum,
    Mean,
    Count,
}

#[derive(Clone, Debug)]
pub(super) struct Channel {
    pub(super) field: Option<String>,
    pub(super) kind: Kind,
    pub(super) aggregate: Option<Aggregate>,
    pub(super) stacked: bool,
}

pub(super) struct Spec {
    pub(super) mark: Mark,
    pub(super) donut: bool,
    pub(super) rows: Vec<Row>,
    pub(super) x: Option<Channel>,
    pub(super) y: Option<Channel>,
    pub(super) color: Option<Channel>,
    pub(super) theta: Option<Channel>,
    pub(super) offset: bool,
    pub(super) title: Option<String>,
    pub(super) width: Option<f32>,
    pub(super) height: Option<f32>,
}

impl Mark {
    fn parse(value: &Value) -> Option<(Self, bool)> {
        let (name, donut) = match value {
            Value::String(name) => (name.as_str(), false),
            Value::Object(map) => (
                map.get("type")?.as_str()?,
                map.get("innerRadius")
                    .and_then(Value::as_f64)
                    .is_some_and(|radius| radius > 0.0),
            ),
            _ => return None,
        };
        let mark = match name {
            "bar" => Self::Bar,
            "line" => Self::Line,
            "area" => Self::Area,
            "point" | "circle" | "square" => Self::Point,
            "arc" => Self::Arc,
            _ => return None,
        };
        Some((mark, donut))
    }
}

impl Kind {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "quantitative" | "Q" => Some(Self::Quantitative),
            "nominal" | "N" => Some(Self::Nominal),
            "ordinal" | "O" => Some(Self::Ordinal),
            "temporal" | "T" => Some(Self::Temporal),
            _ => None,
        }
    }
}

impl Aggregate {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "sum" => Some(Self::Sum),
            "mean" | "average" => Some(Self::Mean),
            "count" => Some(Self::Count),
            _ => None,
        }
    }
}

#[allow(
    clippy::option_option,
    reason = "the outer None rejects the spec and the inner None means the key is absent"
)]
fn optional<T>(
    value: Option<&Value>,
    parse: impl FnOnce(&Value) -> Option<T>,
) -> Option<Option<T>> {
    value.map_or(Some(None), |value| parse(value).map(Some))
}

impl Channel {
    fn parse(value: &Value) -> Option<Self> {
        let map = value.as_object()?;
        if REJECTED.iter().any(|key| map.contains_key(*key)) {
            return None;
        }
        let field = optional(map.get("field"), |field| field.as_str().map(str::to_owned))?;
        let aggregate = optional(map.get("aggregate"), |name| {
            Aggregate::parse(name.as_str()?)
        })?;
        let declared = optional(map.get("type"), |name| Kind::parse(name.as_str()?))?;
        let kind = declared.unwrap_or_else(|| {
            if aggregate.is_some() {
                Kind::Quantitative
            } else {
                Kind::Nominal
            }
        });
        if field.is_none() && aggregate != Some(Aggregate::Count) {
            return None;
        }
        Some(Self {
            field,
            kind,
            aggregate,
            stacked: !matches!(map.get("stack"), Some(Value::Null | Value::Bool(false))),
        })
    }
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

impl Spec {
    pub(super) fn parse(value: &Value) -> Option<Self> {
        let top = value.as_object()?;
        if top.keys().any(|key| !TOP_KEYS.contains(&key.as_str())) {
            return None;
        }
        let (mark, donut) = Mark::parse(top.get("mark")?)?;
        let encoding = top.get("encoding")?.as_object()?;
        if encoding.keys().any(|key| !CHANNELS.contains(&key.as_str())) {
            return None;
        }
        let channel = |name: &str| optional(encoding.get(name), Channel::parse);
        Some(Self {
            mark,
            donut,
            rows: rows(top.get("data")?)?,
            x: channel("x")?,
            y: channel("y")?,
            color: channel("color")?,
            theta: channel("theta")?,
            offset: encoding.contains_key("xOffset"),
            title: optional(top.get("title"), title)?,
            width: size(top.get("width")),
            height: size(top.get("height")),
        })
    }
}
