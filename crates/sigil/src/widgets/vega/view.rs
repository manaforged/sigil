use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::fnv::{OFFSET, mix};
use crate::kernel::{NodeBuilder, Role, container, fragment, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::reactive::{Owner, Signal, signal};
use crate::style::{Metrics, Radius, Theme, Token};
use crate::tasks::current::host_parts;
use crate::tasks::dispatch::spawn_into;
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
    requested: RefCell<HashSet<u64>>,
}

impl Spec {
    fn job(&self, theme: &Theme) -> Job {
        Job::new(&self.source, self.key, config(theme))
    }

    fn fallback(&self) -> NodeBuilder {
        (self.fallback)(&self.source)
    }

    fn request(&self, job: Job, arrived: Signal<u64>, owner: Option<Owner>) -> bool {
        let theme = job.theme();
        if !self.requested.borrow_mut().insert(theme) {
            return true;
        }
        let (Some(owner), Some(parts)) = (owner, host_parts("vega theme")) else {
            return false;
        };
        let target = arrived.into_sendable();
        let started = spawn_into(&parts, owner, engine::render(vec![job]), move |_| {
            target.into_signal().try_set_always(theme);
        });
        started.is_some()
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
        key: source
            .bytes()
            .fold(OFFSET, |key, byte| mix(key, u64::from(byte))),
        name: name(object),
        about: object
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string),
        height: height(object),
        fallback,
        requested: RefCell::default(),
    });
    let jobs = vec![spec.job(&Theme::light()), spec.job(&Theme::dark())];
    if let Some(outcome) = engine::cached(&jobs[0]) {
        return Some(settled(&spec, outcome));
    }
    let status = spawn(engine::render(jobs));
    Some(fragment().children_reactive(move || [shown(&spec, status.get())]))
}

fn shown(spec: &Rc<Spec>, status: Status<Arc<Chart>, Failure>) -> NodeBuilder {
    match status {
        Status::Pending => placeholder(spec.height),
        Status::Ok(chart) => drawn(spec, &chart),
        Status::Idle => settled(spec, engine::render_now(spec.job(&Theme::light()))),
        Status::Err(_) | Status::Panicked(_) => spec.fallback(),
    }
}

fn settled(spec: &Rc<Spec>, outcome: Outcome) -> NodeBuilder {
    match outcome {
        Ok(chart) => drawn(spec, &chart),
        Err(_) => spec.fallback(),
    }
}

fn drawn(spec: &Rc<Spec>, chart: &Chart) -> NodeBuilder {
    let arrived = signal(0_u64);
    let owner = Owner::current();
    let Some((node, (width, _))) = chart_picture(spec, arrived, owner) else {
        return spec.fallback();
    };
    let first = Cell::new(Some(node));
    let again = Rc::clone(spec);
    let shown = fragment().children_reactive(move || {
        arrived.track();
        first
            .take()
            .or_else(|| chart_picture(&again, arrived, owner).map(|(node, _)| node))
    });
    let row = container()
        .layout(LayoutStyle::flex_row().fill_width().min_size(width, 0.0))
        .child(shown);
    Horizontal::new(signal(width), None)
        .wrap(row)
        .role(Role::Figure)
        .label(spec.name.clone())
        .description(description(spec.about.as_deref(), &chart.text))
}

fn chart_picture(
    spec: &Rc<Spec>,
    arrived: Signal<u64>,
    owner: Option<Owner>,
) -> Option<(NodeBuilder, (f32, f32))> {
    let asker = Rc::clone(spec);
    let render = move |theme: &Theme| {
        let job = asker.job(theme);
        let outcome = match engine::cached(&job) {
            Some(outcome) => outcome,
            None if asker.request(job.clone(), arrived, owner) => return None,
            None => engine::render_now(job),
        };
        outcome.ok().map(|chart| chart.svg.clone())
    };
    picture(&spec.source, render)
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
        .map_or(PLOT_HEIGHT, |height| {
            height.clamp(0.0, f64::from(MAX_HEIGHT)) as f32
        })
        + AXIS_HEIGHT
}
