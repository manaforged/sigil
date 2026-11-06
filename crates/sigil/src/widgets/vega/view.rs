use std::rc::Rc;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::fnv::{OFFSET, mix};
use crate::kernel::{NodeBuilder, Role, container, fragment, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::reactive::signal;
use crate::style::{Metrics, Radius, Theme, Token};
use crate::tasks::{Status, spawn};
use crate::widgets::Widget;
use crate::widgets::scroll::Horizontal;
use crate::widgets::source_too_large;
use crate::widgets::svg_picture::picture;

use super::Fallback;
use super::engine::{self, Chart, Failure, Job, Outcome};
use super::theme::config;

const RENDERING: &str = "Rendering chart";
const UNTITLED: &str = "Chart";
const PLOT_HEIGHT: f32 = 300.0;
const AXIS_HEIGHT: f32 = 50.0;
const MAX_HEIGHT: f32 = 1200.0;

struct Spec {
    source: Arc<str>,
    key: u64,
    name: String,
    about: Option<String>,
    height: f32,
    fallback: Fallback,
}

impl Spec {
    fn job(&self, theme: &Theme) -> Job {
        Job::new(&self.source, self.key, config(theme))
    }

    fn fallback(&self) -> NodeBuilder {
        (self.fallback)(&self.source)
    }
}

pub(super) fn figure(source: &str, fallback: Fallback) -> Option<NodeBuilder> {
    if Metrics::current().is_grid() || source_too_large(source) {
        return None;
    }
    let value: Value = serde_json::from_str(source).ok()?;
    let object = value.as_object()?;
    let spec = Rc::new(Spec {
        source: source.into(),
        key: source.bytes().fold(OFFSET, |key, byte| mix(key, u64::from(byte))),
        name: name(object),
        about: object.get("description").and_then(Value::as_str).map(str::to_string),
        height: height(object),
        fallback,
    });
    let jobs = vec![spec.job(&Theme::light()), spec.job(&Theme::dark())];
    if let Some(outcome) = engine::cached(&jobs[0]) {
        return Some(settled(&spec, outcome));
    }
    let status = spawn(engine::render(jobs));
    Some(fragment().children_reactive(move || [shown(&spec, status.get())]))
}

fn shown(spec: &Spec, status: Status<Arc<Chart>, Failure>) -> NodeBuilder {
    match status {
        Status::Pending => placeholder(spec.height),
        Status::Ok(chart) => drawn(spec, &chart),
        Status::Idle => settled(spec, engine::render_now(spec.job(&Theme::light()))),
        Status::Err(_) | Status::Panicked(_) => spec.fallback(),
    }
}

fn settled(spec: &Spec, outcome: Outcome) -> NodeBuilder {
    match outcome {
        Ok(chart) => drawn(spec, &chart),
        Err(_) => spec.fallback(),
    }
}

fn drawn(spec: &Spec, chart: &Chart) -> NodeBuilder {
    let (source, key) = (Arc::clone(&spec.source), spec.key);
    let render = move |theme: &Theme| {
        engine::render_now(Job::new(&source, key, config(theme)))
            .ok()
            .map(|chart| chart.svg.clone())
    };
    let Some((node, (width, _))) = picture(&spec.source, render) else {
        return spec.fallback();
    };
    let row = container()
        .layout(LayoutStyle::flex_row().fill_width().min_size(width, 0.0))
        .child(node);
    Horizontal::new(signal(width), None)
        .wrap(row)
        .role(Role::Figure)
        .label(spec.name.clone())
        .description(description(spec.about.as_deref(), &chart.text))
}

fn placeholder(height: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .height(height)
                .justify_content(JustifyContent::Center)
                .align_items(AlignItems::Center),
        )
        .class([Token::Field.bg(), Radius::Md.atom()])
        .child(
            text(RENDERING)
                .class("text-xs")
                .class(Token::OnSurfaceMuted.text()),
        )
}

fn description(about: Option<&str>, titles: &str) -> String {
    about
        .into_iter()
        .chain(titles.lines())
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn name(spec: &Map<String, Value>) -> String {
    let title = match spec.get("title") {
        Some(Value::Object(title)) => title.get("text"),
        title => title,
    };
    let text = match title {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Array(lines)) => Some(
            lines
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => None,
    };
    text.filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| UNTITLED.to_string())
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the height is clamped to MAX_HEIGHT before the cast"
)]
fn height(spec: &Map<String, Value>) -> f32 {
    spec.get("height")
        .and_then(Value::as_f64)
        .map_or(PLOT_HEIGHT, |height| height.clamp(0.0, f64::from(MAX_HEIGHT)) as f32)
        + AXIS_HEIGHT
}
