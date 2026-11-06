use std::rc::Rc;

use serde_json::{Map, Value};

use crate::kernel::{NodeBuilder, container, text};
use crate::layout::LayoutStyle;
use crate::style::{Token, space};
use crate::widgets::Widget;

use super::palette::Hue;
use super::{ChartBuilder, ChartMark, Stack};

mod spec;
mod table;
mod time;

use spec::{Channel, Kind, Mark, Spec};

const MAX_BYTES: usize = 256 * 1024;
const MAX_WIDTH: f32 = 2000.0;
const MAX_HEIGHT: f32 = 1200.0;

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
    if let Some(height) = spec.height {
        chart = chart.height(height.min(MAX_HEIGHT));
    }
    let Some(title) = &spec.title else {
        return chart.into_node_builder();
    };
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(space(2)))
        .child(
            text(title.clone())
                .wrap()
                .class("text-sm")
                .class("font-bold")
                .class(Token::OnSurface.text()),
        )
        .child(chart)
}

fn build(series: Vec<(String, Vec<f32>)>, mark: ChartMark) -> ChartBuilder {
    series
        .into_iter()
        .enumerate()
        .fold(ChartBuilder::blank(), |chart, (index, (name, values))| {
            chart.push(name, Rc::new(values), Hue::nth(index))
        })
        .mark(mark)
}

fn arc(spec: &Spec) -> Option<ChartBuilder> {
    let theta = spec.theta.as_ref()?;
    let color = spec.color.as_ref()?;
    let table = table::pivot(&spec.rows, color, theta, None)?;
    let values = table.series.into_iter().next()?.1;
    let mark = if spec.donut {
        ChartMark::Donut
    } else {
        ChartMark::Pie
    };
    Some(build(vec![(String::new(), values)], mark).labels(table.labels))
}

fn scatter(spec: &Spec) -> Option<ChartBuilder> {
    let (x, y) = (spec.x.as_ref()?, spec.y.as_ref()?);
    if x.kind != Kind::Quantitative || y.kind != Kind::Quantitative {
        return None;
    }
    let series = table::points(&spec.rows, x, y, spec.color.as_ref())?;
    let flat = series
        .into_iter()
        .map(|(name, rows)| (name, rows.into_iter().flat_map(<[f32; 2]>::from).collect()))
        .collect();
    Some(build(flat, ChartMark::Scatter))
}

fn axes(spec: &Spec) -> Option<(&Channel, &Channel)> {
    let (x, y) = (spec.x.as_ref()?, spec.y.as_ref()?);
    if x.kind == Kind::Quantitative && y.kind != Kind::Quantitative {
        Some((y, x))
    } else {
        Some((x, y))
    }
}

fn columns(spec: &Spec) -> Option<ChartBuilder> {
    let (category, value) = axes(spec)?;
    if value.kind != Kind::Quantitative {
        return None;
    }
    let mark = match spec.mark {
        Mark::Line => ChartMark::Line,
        Mark::Area => ChartMark::Area,
        Mark::Bar | Mark::Point | Mark::Arc => ChartMark::Bar,
    };
    let by_value = category.kind == Kind::Quantitative;
    if by_value && mark == ChartMark::Bar {
        return None;
    }
    let stacked = mark != ChartMark::Line && spec.color.is_some() && value.stacked && !spec.offset;
    let mut table = table::pivot(&spec.rows, category, value, spec.color.as_ref())?;
    let positions = if by_value {
        Some(table.positions()?)
    } else {
        None
    };
    table.fill_missing(if stacked || mark == ChartMark::Bar {
        0.0
    } else {
        f32::NAN
    });
    let mut chart = build(table.series, mark).labels(table.labels);
    if stacked {
        chart.stack = Stack::Stacked;
    }
    chart.positions = positions.map(Rc::from);
    chart.whole = value.aggregate == Some(spec::Aggregate::Count);
    Some(chart)
}
