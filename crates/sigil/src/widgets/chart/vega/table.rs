use std::collections::HashMap;

use serde_json::Value;

use super::Row;
use super::channel::{Aggregate, Channel, Kind, key_text};
use super::order::{self, Facts};
use super::time::epoch_ms;

const MAX_CATEGORIES: usize = 500;
const MAX_SERIES: usize = 10;

pub(super) struct Table {
    pub(super) labels: Vec<String>,
    pub(super) label_slots: Vec<usize>,
    pub(super) series: Vec<(String, Vec<f32>)>,
    pub(super) slots: Vec<usize>,
}

#[derive(Clone, Copy, Default)]
struct Cell {
    sum: f64,
    count: u32,
}

#[derive(Default)]
struct Keys {
    names: Vec<String>,
    nulls: Vec<bool>,
    counts: Vec<u32>,
    index: HashMap<String, usize>,
}

impl Keys {
    fn slot(&mut self, key: (String, bool), limit: usize) -> Option<usize> {
        let (name, null) = key;
        if let Some(at) = self.index.get(&name).copied() {
            if let Some(count) = self.counts.get_mut(at) {
                *count += 1;
            }
            return Some(at);
        }
        if self.names.len() >= limit {
            return None;
        }
        self.index.insert(name.clone(), self.names.len());
        self.names.push(name);
        self.nulls.push(null);
        self.counts.push(1);
        Some(self.names.len() - 1)
    }

    fn rank(&self, channel: Option<&Channel>, totals: &[f64], reorder: bool) -> (Vec<usize>, Vec<usize>) {
        let Some(channel) = channel else {
            return ((0..self.names.len()).collect(), (0..self.names.len()).collect());
        };
        if matches!(channel.kind, Kind::Quantitative | Kind::Temporal) {
            let order = by_place(&self.names, channel.kind);
            let slots = (0..order.len()).collect();
            return (order, slots);
        }
        let facts = Facts {
            names: &self.names,
            nulls: &self.nulls,
            counts: &self.counts,
            totals,
        };
        let ranked = order::rank(&facts, &channel.sort, channel.name);
        order::domain(ranked, &self.names, &channel.scale.domain, reorder)
    }
}

fn key(row: &Row, channel: Option<&Channel>) -> Option<(String, bool)> {
    let Some(channel) = channel else {
        return Some((String::new(), false));
    };
    let value = row.get(channel.field.as_deref()?)?;
    let continuous = matches!(channel.kind, Kind::Quantitative | Kind::Temporal);
    match value {
        Value::Null if continuous => None,
        Value::Null => Some(("null".to_owned(), true)),
        Value::Array(_) | Value::Object(_) => None,
        _ => Some((key_text(value)?, false)),
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

pub(super) struct Pivot<'a> {
    pub(super) category: &'a Channel,
    pub(super) value: &'a Channel,
    pub(super) color: Option<&'a Channel>,
    pub(super) keep_nulls: bool,
}

type Cells = HashMap<(usize, usize), Cell>;

fn gather(rows: &[Row], pivot: &Pivot<'_>) -> Option<(Keys, Keys, Cells)> {
    let (mut labels, mut groups) = (Keys::default(), Keys::default());
    let mut cells = Cells::new();
    for row in rows {
        let (Some(label), Some(group)) = (key(row, Some(pivot.category)), key(row, pivot.color))
        else {
            continue;
        };
        let amount = measure(row, pivot.value)?;
        if amount.is_none() && !pivot.keep_nulls {
            continue;
        }
        let at = (
            groups.slot(group, MAX_SERIES)?,
            labels.slot(label, MAX_CATEGORIES)?,
        );
        if let Some(amount) = amount {
            let cell = cells.entry(at).or_default();
            cell.sum += f64::from(amount);
            cell.count += 1;
        }
    }
    Some((labels, groups, cells))
}

pub(super) fn pivot(rows: &[Row], pivot: &Pivot<'_>) -> Option<Table> {
    let (labels, groups, cells) = gather(rows, pivot)?;
    let aggregate = pivot.value.aggregate;
    let mut totals = vec![0.0_f64; labels.names.len()];
    for ((_, label), cell) in &cells {
        if let (Some(total), Some(value)) = (totals.get_mut(*label), cell.value(aggregate)) {
            *total += f64::from(value);
        }
    }
    let reorder = pivot.category.name != "color";
    let (label_order, label_slots) = labels.rank(Some(pivot.category), &totals, reorder);
    let (group_order, slots) = groups.rank(pivot.color, &[], true);
    let series = group_order
        .iter()
        .map(|group| {
            let values = label_order
                .iter()
                .map(|label| {
                    cells
                        .get(&(*group, *label))
                        .map_or(Some(f32::NAN), |cell| cell.value(aggregate))
                })
                .collect::<Option<Vec<_>>>()?;
            Some((groups.names.get(*group)?.clone(), values))
        })
        .collect::<Option<Vec<_>>>()?;
    let labels = label_order
        .iter()
        .filter_map(|at| labels.names.get(*at).cloned())
        .collect();
    (!series.is_empty()).then_some(Table {
        labels,
        label_slots,
        series,
        slots,
    })
}

pub(super) fn place(name: &str, kind: Kind) -> Option<f64> {
    match kind {
        Kind::Quantitative => name.parse::<f64>().ok().filter(|value| value.is_finite()),
        Kind::Temporal => epoch_ms(name),
        Kind::Nominal | Kind::Ordinal => None,
    }
}

fn by_place(names: &[String], kind: Kind) -> Vec<usize> {
    let mut order = (0..names.len()).collect::<Vec<_>>();
    let at = names
        .iter()
        .map(|name| place(name, kind).unwrap_or(f64::NAN))
        .collect::<Vec<_>>();
    let key = |index: &usize| at.get(*index).copied().unwrap_or(f64::NAN);
    order.sort_by(|a, b| key(a).total_cmp(&key(b)));
    order
}

impl Table {
    pub(super) fn positions(&self, kind: Kind) -> Option<Vec<f64>> {
        self.labels.iter().map(|name| place(name, kind)).collect()
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
) -> Option<(Points, Vec<usize>)> {
    let mut groups = Keys::default();
    let mut series: Points = Vec::new();
    for row in rows {
        let Some(group) = key(row, color) else {
            continue;
        };
        let (Some(px), Some(py)) = (measure(row, x)?, measure(row, y)?) else {
            continue;
        };
        let name = group.0.clone();
        let at = groups.slot(group, MAX_SERIES)?;
        if at == series.len() {
            series.push((name, Vec::new()));
        }
        series.get_mut(at)?.1.push((px, py));
    }
    let (order, slots) = groups.rank(color, &[], true);
    let mut taken = series.into_iter().map(Some).collect::<Vec<_>>();
    let sorted = order
        .iter()
        .filter_map(|at| taken.get_mut(*at).and_then(Option::take))
        .collect::<Points>();
    (!sorted.is_empty()).then_some((sorted, slots))
}
