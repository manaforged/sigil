use std::rc::Rc;

use super::ChartView;
use super::axis::format_tick;
use super::decimate::Candle;
use super::scatter::{Spans, normalized, pairs};
use super::tips::{Pick, Tips};

fn named(legend: &str, value: String) -> String {
    if legend.is_empty() {
        value
    } else {
        format!("{legend}: {value}")
    }
}

pub(super) fn columns(series: &[ChartView<'_>], labels: &[String], count: usize) -> Tips {
    let labels = labels.to_vec();
    let rows = series
        .iter()
        .map(|item| (item.legend.clone(), item.values.to_vec()))
        .collect::<Vec<_>>();
    Tips {
        pick: Pick::Columns(count),
        lines: Rc::new(move |index| {
            let header = labels.get(index).cloned().into_iter();
            let values = rows.iter().filter_map(|(legend, values)| {
                let value = values.get(index).filter(|value| value.is_finite())?;
                Some(named(legend, format_tick(*value)))
            });
            header.chain(values).collect()
        }),
    }
}

pub(super) fn placed(series: &[ChartView<'_>], labels: &[String], fractions: Rc<[f32]>) -> Tips {
    let mut tips = columns(series, labels, fractions.len());
    tips.pick = Pick::Placed(fractions);
    tips
}

pub(super) fn candles(series: &[ChartView<'_>], labels: &[String]) -> Tips {
    let rows = series
        .first()
        .map(|item| Candle::rows(item.values))
        .unwrap_or_default();
    let labels = labels.to_vec();
    Tips {
        pick: Pick::Columns(rows.len()),
        lines: Rc::new(move |index| {
            let Some(row) = rows.get(index) else {
                return Vec::new();
            };
            let header = labels.get(index).cloned().into_iter();
            let quote = format!(
                "O {} H {} L {} C {}",
                format_tick(row.open),
                format_tick(row.high),
                format_tick(row.low),
                format_tick(row.close)
            );
            header.chain([quote]).collect()
        }),
    }
}

pub(super) fn points(series: &[ChartView<'_>], spans: Spans) -> Tips {
    let rows = series
        .iter()
        .flat_map(|item| pairs(item.values).map(|(x, y)| (item.legend.clone(), x, y)))
        .collect::<Vec<_>>();
    Tips {
        pick: Pick::Points(normalized(series, spans)),
        lines: Rc::new(move |index| {
            rows.get(index).map_or_else(Vec::new, |(legend, x, y)| {
                vec![named(
                    legend,
                    format!("{}, {}", format_tick(*x), format_tick(*y)),
                )]
            })
        }),
    }
}
