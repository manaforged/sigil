use std::collections::HashMap;

use serde_json::Value;

use super::Row;
use super::spec::{Aggregate, Channel, Kind};
use super::time::epoch_ms;

const MAX_CATEGORIES: usize = 500;
const MAX_SERIES: usize = 10;

pub(super) struct Table {
    pub(super) labels: Vec<String>,
    pub(super) series: Vec<(String, Vec<f32>)>,
}

#[derive(Clone, Copy, Default)]
struct Cell {
    sum: f64,
    count: u32,
}

#[derive(Default)]
struct Keys {
    names: Vec<String>,
    index: HashMap<String, usize>,
}

impl Keys {
    fn slot(&mut self, name: String, limit: usize) -> Option<usize> {
        if let Some(at) = self.index.get(&name) {
            return Some(*at);
        }
        if self.names.len() >= limit {
            return None;
        }
        self.index.insert(name.clone(), self.names.len());
        self.names.push(name);
        Some(self.names.len() - 1)
    }
}

fn key(row: &Row, channel: Option<&Channel>) -> Option<String> {
    let Some(field) = channel.and_then(|channel| channel.field.as_deref()) else {
        return Some(String::new());
    };
    match row.get(field)? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "chart values render at f32 precision"
)]
#[allow(
    clippy::option_option,
    reason = "the outer None rejects the row and the inner None means the value is absent"
)]
fn number(value: Option<&Value>) -> Option<Option<f32>> {
    let parsed = match value {
        None | Some(Value::Null) => return Some(None),
        Some(Value::Number(number)) => number.as_f64()? as f32,
        Some(Value::String(text)) => text.trim().parse::<f32>().ok()?,
        Some(_) => return None,
    };
    parsed.is_finite().then_some(Some(parsed))
}

#[allow(
    clippy::option_option,
    reason = "the outer None rejects the row and the inner None means the value is absent"
)]
fn measure(row: &Row, channel: &Channel) -> Option<Option<f32>> {
    if channel.aggregate == Some(Aggregate::Count) {
        return Some(Some(1.0));
    }
    number(row.get(channel.field.as_deref()?))
}

impl Cell {
    #[allow(
        clippy::cast_precision_loss,
        reason = "row counts are bounded by the row limit"
    )]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "chart values render at f32 precision and overflow is rejected"
    )]
    fn value(self, aggregate: Option<Aggregate>) -> Option<f32> {
        let value = match aggregate {
            Some(Aggregate::Mean) if self.count > 0 => self.sum / f64::from(self.count),
            Some(Aggregate::Count) => f64::from(self.count),
            _ => self.sum,
        } as f32;
        value.is_finite().then_some(value)
    }
}

pub(super) fn pivot(
    rows: &[Row],
    category: &Channel,
    value: &Channel,
    color: Option<&Channel>,
) -> Option<Table> {
    let (mut labels, mut groups) = (Keys::default(), Keys::default());
    let mut cells: HashMap<(usize, usize), Cell> = HashMap::new();
    for row in rows {
        let (Some(label), Some(group)) = (key(row, Some(category)), key(row, color)) else {
            continue;
        };
        let Some(amount) = measure(row, value)? else {
            continue;
        };
        let at = (
            groups.slot(group, MAX_SERIES)?,
            labels.slot(label, MAX_CATEGORIES)?,
        );
        let cell = cells.entry(at).or_default();
        cell.sum += f64::from(amount);
        cell.count += 1;
    }
    let order = order(&labels.names, category.kind)?;
    let series = groups
        .names
        .into_iter()
        .enumerate()
        .map(|(group, name)| {
            let values = order
                .iter()
                .map(|label| {
                    cells
                        .get(&(group, *label))
                        .map_or(Some(f32::NAN), |cell| cell.value(value.aggregate))
                })
                .collect::<Option<Vec<_>>>()?;
            Some((name, values))
        })
        .collect::<Option<Vec<_>>>()?;
    let labels = order
        .iter()
        .filter_map(|at| labels.names.get(*at).cloned())
        .collect();
    (!series.is_empty()).then_some(Table { labels, series })
}

fn place(name: &str, kind: Kind) -> Option<f64> {
    match kind {
        Kind::Quantitative => name.parse::<f64>().ok().filter(|value| value.is_finite()),
        Kind::Temporal => epoch_ms(name),
        Kind::Nominal | Kind::Ordinal => None,
    }
}

fn order(names: &[String], kind: Kind) -> Option<Vec<usize>> {
    let mut order = (0..names.len()).collect::<Vec<_>>();
    let at = match kind {
        Kind::Nominal | Kind::Ordinal => return Some(order),
        Kind::Quantitative => names
            .iter()
            .map(|name| place(name, kind).unwrap_or(f64::NAN))
            .collect::<Vec<_>>(),
        Kind::Temporal => names
            .iter()
            .map(|name| place(name, kind))
            .collect::<Option<Vec<_>>>()?,
    };
    let key = |index: &usize| at.get(*index).copied().unwrap_or(f64::NAN);
    order.sort_by(|a, b| key(a).total_cmp(&key(b)));
    Some(order)
}

impl Table {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "chart positions render at f32 precision"
    )]
    pub(super) fn positions(&self) -> Option<Vec<f32>> {
        self.labels
            .iter()
            .map(|name| {
                let value = place(name, Kind::Quantitative)? as f32;
                value.is_finite().then_some(value)
            })
            .collect()
    }

    pub(super) fn fill_missing(&mut self, gap: f32) {
        for (_, values) in &mut self.series {
            for value in values.iter_mut().filter(|value| value.is_nan()) {
                *value = gap;
            }
        }
    }
}

pub(super) type Points = Vec<(String, Vec<(f32, f32)>)>;

pub(super) fn points(
    rows: &[Row],
    x: &Channel,
    y: &Channel,
    color: Option<&Channel>,
) -> Option<Points> {
    let mut groups = Keys::default();
    let mut series: Points = Vec::new();
    for row in rows {
        let Some(group) = key(row, color) else {
            continue;
        };
        let (Some(px), Some(py)) = (measure(row, x)?, measure(row, y)?) else {
            continue;
        };
        let at = groups.slot(group.clone(), MAX_SERIES)?;
        if at == series.len() {
            series.push((group, Vec::new()));
        }
        series.get_mut(at)?.1.push((px, py));
    }
    (!series.is_empty()).then_some(series)
}
