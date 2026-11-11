use crate::layout::Space;
use std::rc::Rc;

use serde_json::{Map, Value};

use crate::kernel::{NodeBuilder, container, text};
use crate::layout::LayoutStyle;
use crate::style::Token;
use crate::widgets::vega::{MAX_HEIGHT, PLOT_HEIGHT};
use crate::widgets::{Layout, Widget};

use super::palette::Hue;
use super::scales::{Fit, Orient};
use super::{ChartBuilder, ChartMark, Stack, clock};

mod channel;
mod color;
mod order;
mod spec;
mod table;
mod time;

use channel::{Aggregate, Channel, Domain, Kind};
use spec::{Mark, Spec};
use table::Pivot;

const MAX_BYTES: usize = 256 * 1024;
const MAX_WIDTH: f32 = 2000.0;

type Row = Map<String, Value>;

pub(crate) fn vega_lite(json: &str) -> Option<NodeBuilder> {
    if json.len() > MAX_BYTES {
        return None;
    }
    let value: Value = serde_json::from_str(json).ok()?;
    let spec = Spec::parse(&value)?;
    let chart = match spec.mark {
        Mark::Arc => arc(&spec)?,
        Mark::Point => scatter(&spec)?,
        Mark::Bar | Mark::Line | Mark::Area => columns(&spec)?,
    };
    Some(frame(&spec, chart.tooltip()))
}

fn frame(spec: &Spec, mut chart: ChartBuilder) -> NodeBuilder {
    if let Some(width) = spec.width {
        chart = chart.width(width.min(MAX_WIDTH));
    }
    chart.scales.plot_height = Some(spec.height.map_or(PLOT_HEIGHT, |px| px.min(MAX_HEIGHT)));
    let Some(title) = &spec.title else {
        return chart.into_node_builder();
    };
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(Space::Sm))
        .child(
            text(title.clone())
                .wrap()
                .class("text-sm")
                .class("font-bold")
                .class(Token::OnSurface.text()),
        )
        .child(chart)
}

fn hues(names: &[String], slots: &[usize], color: Option<&Channel>) -> Vec<Hue> {
    names
        .iter()
        .zip(slots)
        .map(|(name, slot)| {
            let scale = color.map(|color| &color.scale);
            let by_name = scale
                .filter(|scale| scale.disabled)
                .and_then(|_| color::css(name));
            let ranged = scale
                .filter(|scale| !scale.range.is_empty())
                .and_then(|scale| color::css(scale.range.get(slot % scale.range.len())?));
            by_name.or(ranged).map_or_else(|| Hue::nth(*slot), Hue::rgb)
        })
        .collect()
}

fn build(series: Vec<(String, Vec<f32>)>, hues: &[Hue], mark: ChartMark) -> ChartBuilder {
    series
        .into_iter()
        .enumerate()
        .fold(ChartBuilder::blank(), |chart, (index, (name, values))| {
            let hue = hues.get(index).copied().unwrap_or_else(|| Hue::nth(index));
            chart.push(name, Rc::new(values), hue)
        })
        .mark(mark)
}

fn arc(spec: &Spec) -> Option<ChartBuilder> {
    let theta = spec.theta.as_ref()?;
    let color = spec.color.as_ref()?;
    let pivot = Pivot {
        category: color,
        value: theta,
        color: None,
        keep_nulls: false,
    };
    let table = table::pivot(&spec.rows, &pivot)?;
    let slices = hues(&table.labels, &table.label_slots, Some(color));
    let values = table.series.into_iter().next()?.1;
    let mark = if spec.donut {
        ChartMark::Donut
    } else {
        ChartMark::Pie
    };
    let mut chart = build(vec![(String::new(), values)], &[], mark).labels(table.labels);
    chart.scales.slices = Some(Rc::from(slices));
    Some(chart)
}

fn fit(channel: &Channel, zero: Option<bool>) -> Fit {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a chart domain renders at f32 precision"
    )]
    let domain = match channel.scale.domain {
        Domain::Span(lo, hi) => Some((lo as f32, hi as f32)),
        _ => None,
    };
    Fit {
        domain,
        zero: channel.scale.zero.or(zero),
    }
}

fn scatter(spec: &Spec) -> Option<ChartBuilder> {
    let (x, y) = (spec.x.as_ref()?, spec.y.as_ref()?);
    if x.kind != Kind::Quantitative || y.kind != Kind::Quantitative {
        return None;
    }
    let (series, slots) = table::points(&spec.rows, x, y, spec.color.as_ref())?;
    let names = series
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let flat = series
        .into_iter()
        .map(|(name, rows)| (name, rows.into_iter().flat_map(<[f32; 2]>::from).collect()))
        .collect();
    let mut chart = build(
        flat,
        &hues(&names, &slots, spec.color.as_ref()),
        ChartMark::Scatter,
    );
    let zero = Some(spec.zero.unwrap_or(true));
    chart.scales.x = fit(x, zero);
    chart.scales.y = fit(y, zero);
    chart.scales.dot = spec.dot;
    chart.scales.x_title.clone_from(&x.title);
    chart.scales.y_title.clone_from(&y.title);
    Some(chart)
}

fn axes(spec: &Spec) -> Option<(&Channel, &Channel, Orient)> {
    let (x, y) = (spec.x.as_ref()?, spec.y.as_ref()?);
    if x.kind == Kind::Quantitative && y.kind != Kind::Quantitative {
        Some((y, x, Orient::Horizontal))
    } else {
        Some((x, y, Orient::Vertical))
    }
}

const fn chart_mark(mark: Mark) -> ChartMark {
    match mark {
        Mark::Line => ChartMark::Line,
        Mark::Area => ChartMark::Area,
        Mark::Bar | Mark::Point | Mark::Arc => ChartMark::Bar,
    }
}

fn columns(spec: &Spec) -> Option<ChartBuilder> {
    let (category, value, orient) = axes(spec)?;
    let mark = chart_mark(spec.mark);
    let by_value = accepts(category, value, mark)?;
    let stacked = mark != ChartMark::Line && spec.color.is_some() && value.stacked && !spec.offset;
    let pivot = Pivot {
        category,
        value,
        color: spec.color.as_ref(),
        keep_nulls: mark != ChartMark::Bar,
    };
    let mut table = table::pivot(&spec.rows, &pivot)?;
    let positions = by_value.then(|| table.positions(category.kind)).flatten();
    if by_value && positions.is_none() {
        return None;
    }
    table.fill_missing(if stacked || mark == ChartMark::Bar {
        0.0
    } else {
        f32::NAN
    });
    let names = table
        .series
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let hues = hues(&names, &table.slots, spec.color.as_ref());
    let labels = shown_labels(&table.labels, category.kind);
    let mut chart = build(table.series, &hues, mark).labels(labels);
    if stacked {
        chart.stack = Stack::Stacked;
    }
    chart.whole = value.aggregate == Some(Aggregate::Count);
    place(&mut chart, positions, category);
    chart.scales.y = fit(value, spec.zero);
    chart.scales.orient = orient;
    titles(&mut chart, category, value);
    Some(chart)
}

fn accepts(category: &Channel, value: &Channel, mark: ChartMark) -> Option<bool> {
    let continuous = matches!(category.kind, Kind::Quantitative | Kind::Temporal);
    let bars = mark == ChartMark::Bar;
    let numeric_bars = bars && category.kind == Kind::Quantitative;
    (value.kind == Kind::Quantitative && !numeric_bars).then_some(continuous && !bars)
}

fn titles(chart: &mut ChartBuilder, category: &Channel, value: &Channel) {
    let (side, foot) = match chart.scales.orient {
        Orient::Vertical => (&value.title, &category.title),
        Orient::Horizontal => (&category.title, &value.title),
    };
    chart.scales.y_title.clone_from(side);
    chart.scales.x_title.clone_from(foot);
}

fn shown_labels(labels: &[String], kind: Kind) -> Vec<String> {
    labels
        .iter()
        .map(|label| match kind {
            Kind::Temporal => time::epoch_ms(label).map_or_else(|| label.clone(), clock::format),
            _ => label.clone(),
        })
        .collect()
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "chart positions render at f32 precision as offsets from the first instant"
)]
fn place(chart: &mut ChartBuilder, positions: Option<Vec<f64>>, category: &Channel) {
    let Some(positions) = positions else {
        return;
    };
    let temporal = category.kind == Kind::Temporal;
    let origin = if temporal {
        positions.iter().copied().fold(f64::INFINITY, f64::min)
    } else {
        0.0
    };
    let mut fit = fit(category, Some(false));
    if let (true, Domain::Span(lo, hi)) = (temporal, &category.scale.domain) {
        fit.domain = Some(((lo - origin) as f32, (hi - origin) as f32));
    }
    chart.scales.x = fit;
    chart.scales.time = temporal.then_some(origin);
    chart.positions = Some(positions.iter().map(|at| (at - origin) as f32).collect());
}
