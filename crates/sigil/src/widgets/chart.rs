use crate::layout::audit::Flaw;
use crate::style::theme::ambient::ActiveTheme;
use crate::widgets::widget::Widget as _;
use std::ops::Range;
use std::rc::Rc;

use crate::kernel::{AccessibleName, NodeBuilder};
use crate::reactive::{Reactive, Signal, memo};
use crate::style::{Style, Tone};
use crate::widgets::LayoutEdit;

use super::layout_ext::LayoutOverrides;
use palette::Hue;

mod axis;
mod bars;
mod clock;
mod decimate;
mod figure;
mod interact;
mod layers;
mod marks;
mod palette;
mod pie;
mod plot;
mod readout;
mod scales;
mod scatter;
mod span;
mod summary;
mod ticks;
mod tips;
mod vega;

#[cfg(feature = "scene")]
pub(crate) use palette::series_tokens;
pub(crate) use vega::vega_lite;

const DEFAULT_HEIGHT: f32 = 180.0;
const MIN_HEIGHT: f32 = 48.0;
const MIN_WIDTH: f32 = 80.0;
const PERCENT: f32 = 100.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ChartMark {
    #[default]
    Bar,
    Line,
    Area,
    Scatter,
    Pie,
    Donut,
    Candle,
}

impl ChartMark {
    const fn stride(self) -> usize {
        match self {
            Self::Candle => 4,
            Self::Scatter => 2,
            Self::Bar | Self::Line | Self::Area | Self::Pie | Self::Donut => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Stack {
    #[default]
    Grouped,
    Stacked,
}

#[derive(Clone, PartialEq)]
pub(super) struct ChartSeries {
    name: String,
    values: Vec<f32>,
    hue: Hue,
}

pub(super) struct ChartView<'a> {
    name: &'a str,
    legend: String,
    values: &'a [f32],
    hue: Hue,
}

struct SeriesSource {
    name: String,
    values: Rc<dyn Reactive<Vec<f32>>>,
    hue: Hue,
}

/// Builder of a chart.
#[must_use]
pub struct ChartBuilder {
    series: Vec<SeriesSource>,
    labels: Vec<String>,
    mark: ChartMark,
    stack: Stack,
    x_range: Option<Signal<Range<f32>>>,
    crosshair: bool,
    tooltip: bool,
    whole: bool,
    positions: Option<Rc<[f32]>>,
    scales: scales::Scales,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

impl core::fmt::Debug for ChartBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ChartBuilder")
            .field("labels", &self.labels)
            .field("mark", &self.mark)
            .field("stack", &self.stack)
            .field("crosshair", &self.crosshair)
            .field("tooltip", &self.tooltip)
            .field("whole", &self.whole)
            .finish_non_exhaustive()
    }
}

/// A bar, line, area, scatter, pie, or candle chart.
pub fn chart(values: impl Reactive<Vec<f32>>) -> ChartBuilder {
    ChartBuilder::blank().push(String::new(), Rc::new(values), Hue::of(Tone::Primary))
}

fn flat_pairs(rows: impl Reactive<Vec<(f32, f32)>>) -> impl Reactive<Vec<f32>> {
    move || {
        rows.read()
            .into_iter()
            .flat_map(<[f32; 2]>::from)
            .collect::<Vec<f32>>()
    }
}

impl ChartBuilder {
    fn blank() -> Self {
        Self {
            series: Vec::new(),
            labels: Vec::new(),
            mark: ChartMark::Bar,
            stack: Stack::Grouped,
            x_range: None,
            crosshair: false,
            tooltip: false,
            whole: false,
            positions: None,
            scales: scales::Scales::default(),
            layout_overrides: LayoutOverrides::new(),
            style_accumulator: Style::new(),
        }
    }

    fn push(mut self, name: String, values: Rc<dyn Reactive<Vec<f32>>>, hue: Hue) -> Self {
        self.series.push(SeriesSource { name, values, hue });
        self
    }

    const fn mark(mut self, mark: ChartMark) -> Self {
        self.mark = mark;
        self
    }

    /// Creates a candlestick chart from open, high, low, and close rows.
    pub fn candles(rows: impl Reactive<Vec<(f32, f32, f32, f32)>>) -> Self {
        let flat = move || {
            rows.read()
                .into_iter()
                .flat_map(<[f32; 4]>::from)
                .collect::<Vec<f32>>()
        };
        chart(flat).mark(ChartMark::Candle)
    }

    /// Creates a scatter chart from x and y rows.
    pub fn scatter(rows: impl Reactive<Vec<(f32, f32)>>) -> Self {
        chart(flat_pairs(rows)).mark(ChartMark::Scatter)
    }

    /// Adds a named series of x and y rows to a scatter chart.
    pub fn points(
        self,
        name: impl Into<String>,
        rows: impl Reactive<Vec<(f32, f32)>>,
        tone: Tone,
    ) -> Self {
        self.push(name.into(), Rc::new(flat_pairs(rows)), Hue::of(tone))
    }

    /// Sets the x range.
    pub const fn x_range(mut self, range: Signal<Range<f32>>) -> Self {
        self.x_range = Some(range);
        self
    }

    /// Applies the crosshair option.
    pub const fn crosshair(mut self) -> Self {
        self.crosshair = true;
        self
    }

    /// Shows the values under the pointer in a card.
    pub const fn tooltip(mut self) -> Self {
        self.tooltip = true;
        self
    }

    /// Applies the whole numbers option.
    pub const fn whole_numbers(mut self) -> Self {
        self.whole = true;
        self
    }

    /// Applies the bar option.
    pub const fn bar(self) -> Self {
        self.mark(ChartMark::Bar)
    }

    /// Stacks the bars of each series on one column.
    pub const fn stacked(mut self) -> Self {
        self.stack = Stack::Stacked;
        self.mark(ChartMark::Bar)
    }

    /// Lays the bars along rows, with values on the x axis.
    pub const fn horizontal(mut self) -> Self {
        self.scales.orient = scales::Orient::Horizontal;
        self.mark(ChartMark::Bar)
    }

    /// Applies the line option.
    pub const fn line(self) -> Self {
        self.mark(ChartMark::Line)
    }

    /// Fills the area under each line.
    pub const fn area(self) -> Self {
        self.mark(ChartMark::Area)
    }

    /// Draws the first series as pie slices named by the labels.
    pub const fn pie(self) -> Self {
        self.mark(ChartMark::Pie)
    }

    /// Draws the first series as a ring of slices named by the labels.
    pub const fn donut(self) -> Self {
        self.mark(ChartMark::Donut)
    }

    /// Sets the tone.
    pub fn tone(mut self, tone: Tone) -> Self {
        if let Some(series) = self.series.first_mut() {
            series.hue = Hue::of(tone);
        }
        self
    }

    /// Sets the labels.
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the series.
    pub fn series(
        self,
        name: impl Into<String>,
        values: impl Reactive<Vec<f32>>,
        tone: Tone,
    ) -> Self {
        self.push(name.into(), Rc::new(values), Hue::of(tone))
    }

    fn titles(&self) -> u8 {
        u8::from(self.scales.x_title.is_some()) + u8::from(self.scales.y_title.is_some())
    }

    fn natural_height(&self) -> f32 {
        let Some(plot) = self.scales.plot_height else {
            return DEFAULT_HEIGHT;
        };
        let legend = self.series.len() > 1 || self.series.iter().any(|item| !item.name.is_empty());
        let chrome = if matches!(self.mark, ChartMark::Pie | ChartMark::Donut) {
            0.0
        } else {
            plot::PlotFrame::chrome(self.titles(), legend)
        };
        plot + chrome
    }

    fn frame(&mut self) -> plot::PlotFrame {
        let width = self
            .layout_overrides
            .width_hint()
            .map(|px| px.max(MIN_WIDTH));
        let height = self
            .layout_overrides
            .height_hint()
            .unwrap_or_else(|| self.natural_height())
            .max(MIN_HEIGHT);
        if let Some(px) = width {
            self.layout_overrides.edit(LayoutEdit::Width(px.into()));
        }
        self.layout_overrides
            .edit(LayoutEdit::Height(height.into()));
        plot::PlotFrame {
            width,
            height,
            whole: self.whole,
            titles: self.titles(),
        }
    }

    fn root(self) -> NodeBuilder {
        const REASON: &str = "marks and axis labels sit where the data puts them";
        self.plot()
            .allow_flaw(Flaw::Inset, REASON)
            .allow_flaw(Flaw::Column, REASON)
            .allow_flaw(Flaw::Overflow, REASON)
            .allow_flaw(Flaw::Overlap, REASON)
    }

    fn plot(mut self) -> NodeBuilder {
        let theme = ActiveTheme::capture();
        let look = figure::Look {
            mark: self.mark,
            stack: self.stack,
            frame: self.frame(),
            theme,
            scales: std::mem::take(&mut self.scales),
        };
        let sources = std::mem::take(&mut self.series);
        let series = memo(move || {
            sources
                .iter()
                .map(|source| ChartSeries {
                    name: source.name.clone(),
                    values: source.values.read(),
                    hue: source.hue,
                })
                .collect::<Vec<_>>()
        });
        let labels = Rc::new(std::mem::take(&mut self.labels));
        let positions = self.positions.take();
        let label_names = Rc::clone(&labels);
        let description =
            memo(move || series.with(|series| summary::summary(series, &label_names, theme)));
        let interact = interact::Interact::new(self.x_range, self.crosshair, self.tooltip);
        let paint = std::mem::replace(&mut self.style_accumulator, Style::new());
        let root = figure::frame(look.frame).mutate(move |target| {
            target.a11y.description = Some(AccessibleName::Derived(description));
        });
        super::shared::caller_style(root, paint).children_reactive(move || {
            series.with(|series| {
                figure::content(series, &labels, positions.as_deref(), &look, &interact)
            })
        })
    }
}

builder_impls!([] ChartBuilder: paint(style_accumulator), widget, node);

pub(crate) fn nice_max(max: f32) -> f32 {
    if !max.is_finite() || max <= 0.0 {
        return 1.0;
    }
    let exp = 10f32.powf(max.log10().floor());
    let n = max / exp;
    let nice = [1.0, 2.0, 5.0].into_iter().find(|&step| n <= step);
    nice.unwrap_or(10.0) * exp
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod tests;
