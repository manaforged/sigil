use serde_json::{Map, Value};

use super::spec::optional;

const REJECTED: [&str; 6] = ["bin", "timeUnit", "value", "datum", "condition", "impute"];

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

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum Sort {
    #[default]
    Ascending,
    Descending,
    Unsorted,
    Order(Vec<String>),
    Channel(String, bool),
    Count(bool),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum Domain {
    #[default]
    Data,
    Span(f64, f64),
    Values(Vec<String>),
    Union(Vec<String>),
}

#[derive(Clone, Debug, Default)]
pub(super) struct Scale {
    pub(super) domain: Domain,
    pub(super) range: Vec<String>,
    pub(super) zero: Option<bool>,
    pub(super) disabled: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Channel {
    pub(super) name: &'static str,
    pub(super) field: Option<String>,
    pub(super) kind: Kind,
    pub(super) aggregate: Option<Aggregate>,
    pub(super) stacked: bool,
    pub(super) sort: Sort,
    pub(super) scale: Scale,
    pub(super) title: Option<String>,
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

pub(super) fn key_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Null => Some("null".to_owned()),
        _ => None,
    }
}

fn keys(values: &[Value]) -> Option<Vec<String>> {
    values.iter().map(key_text).collect()
}

impl Sort {
    fn parse(value: &Value) -> Option<Self> {
        match value {
            Value::Null => Some(Self::Unsorted),
            Value::Array(values) => keys(values).map(Self::Order),
            Value::String(name) => Some(match name.as_str() {
                "ascending" => Self::Ascending,
                "descending" => Self::Descending,
                other => {
                    let channel = other.strip_prefix('-');
                    Self::Channel(channel.unwrap_or(other).to_owned(), channel.is_some())
                }
            }),
            Value::Object(map) => Self::by_field(map),
            _ => None,
        }
    }

    fn by_field(map: &Map<String, Value>) -> Option<Self> {
        let descending = match map.get("order") {
            None => false,
            Some(Value::Null) => return Some(Self::Unsorted),
            Some(order) => order.as_str()? == "descending",
        };
        if map.get("op").and_then(Value::as_str) == Some("count") {
            return Some(Self::Count(descending));
        }
        Some(Self::Channel(String::from("value"), descending))
    }
}

impl Domain {
    fn parse(value: &Value) -> Option<Self> {
        match value {
            Value::Array(values) => match values.as_slice() {
                [Value::Number(lo), Value::Number(hi)] => {
                    Some(Self::Span(lo.as_f64()?, hi.as_f64()?))
                }
                _ => keys(values).map(Self::Values),
            },
            Value::Object(map) => keys(map.get("unionWith")?.as_array()?).map(Self::Union),
            _ => None,
        }
    }
}

impl Scale {
    fn parse(value: Option<&Value>) -> Option<Self> {
        let map = match value {
            None => return Some(Self::default()),
            Some(Value::Null) => {
                return Some(Self {
                    disabled: true,
                    ..Self::default()
                });
            }
            Some(value) => value.as_object()?,
        };
        let range = optional(map.get("range"), |range| {
            range
                .as_array()?
                .iter()
                .map(|color| color.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })?;
        Some(Self {
            domain: optional(map.get("domain"), Domain::parse)?.unwrap_or_default(),
            range: range.unwrap_or_default(),
            zero: map.get("zero").and_then(Value::as_bool),
            disabled: false,
        })
    }
}

fn title(map: &Map<String, Value>, field: Option<&str>, aggregate: Option<&str>) -> Option<String> {
    let axis = map.get("axis");
    if matches!(axis, Some(Value::Null)) {
        return None;
    }
    let written = map
        .get("title")
        .or_else(|| axis.and_then(|axis| axis.get("title")));
    if let Some(written) = written {
        return written
            .as_str()
            .filter(|text| !text.is_empty())
            .map(str::to_owned);
    }
    match (aggregate, field) {
        (Some("count"), _) => Some("Count of Records".to_owned()),
        (Some(op), Some(field)) => {
            let mut letters = op.chars();
            let head = letters.next().map(|first| first.to_ascii_uppercase());
            Some(format!(
                "{}{} of {field}",
                head.unwrap_or_default(),
                letters.as_str()
            ))
        }
        (None, Some(field)) => Some(field.to_owned()),
        (_, None) => None,
    }
}

impl Channel {
    pub(super) fn parse(name: &'static str, value: &Value) -> Option<Self> {
        let map = value.as_object()?;
        if REJECTED.iter().any(|key| map.contains_key(*key)) {
            return None;
        }
        let field = optional(map.get("field"), |field| field.as_str().map(str::to_owned))?;
        let op = optional(map.get("aggregate"), |name| {
            name.as_str().map(str::to_owned)
        })?;
        let aggregate = optional(map.get("aggregate"), |name| {
            Aggregate::parse(name.as_str()?)
        })?;
        let declared = optional(map.get("type"), |name| Kind::parse(name.as_str()?))?;
        let kind = declared.unwrap_or(if aggregate.is_some() {
            Kind::Quantitative
        } else {
            Kind::Nominal
        });
        if field.is_none() && aggregate != Some(Aggregate::Count) {
            return None;
        }
        Some(Self {
            name,
            title: title(map, field.as_deref(), op.as_deref()),
            field,
            kind,
            aggregate,
            stacked: !matches!(map.get("stack"), Some(Value::Null | Value::Bool(false))),
            sort: optional(map.get("sort"), Sort::parse)?.unwrap_or_default(),
            scale: Scale::parse(map.get("scale"))?,
        })
    }
}
