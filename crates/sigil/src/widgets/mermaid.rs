use std::cell::RefCell;
use std::rc::Rc;

use crate::kernel::{NodeBuilder, Role, container, fragment, text};
use crate::layout::LayoutStyle;
use crate::layout::snap::CellForm;
use crate::reactive::{Reactive, signal};
use crate::widgets::Widget;
use crate::widgets::scroll::Horizontal;
pub(super) use crate::widgets::scrolled_diagram as scrolled;
use crate::widgets::{bound_source, frame_source, lines_extent};

mod canvas;
#[cfg(feature = "scene")]
mod chrome;
#[cfg(feature = "scene")]
mod fit;
mod flow;
mod flow_layout;
mod flow_paint;
mod flow_parse;
mod flow_route;
mod flow_vector;
mod label;
#[cfg(feature = "scene")]
mod lightbox;
#[cfg(feature = "scene")]
mod picture;
mod pie;
mod rank;
mod sequence;
mod state_parse;
mod strokes;

/// A rendered mermaid diagram.
pub fn mermaid(source: impl Reactive<String>) -> NodeBuilder {
    let view = View::new();
    fragment().children_reactive(move || [view.figure(&source.read())])
}

#[cfg(any(test, fuzzing))]
pub(crate) fn figure(source: &str) -> NodeBuilder {
    View::new().figure(source)
}

type Draw = fn(&str) -> Option<Drawn>;

struct Drawn {
    node: NodeBuilder,
    width: f32,
    #[cfg(feature = "scene")]
    height: Option<f32>,
    #[cfg(feature = "scene")]
    source: String,
}

#[derive(Clone)]
struct View {
    last: Rc<RefCell<Option<String>>>,
    #[cfg(feature = "scene")]
    chrome: chrome::Chrome,
}

impl View {
    fn new() -> Self {
        Self {
            last: Rc::new(RefCell::new(None)),
            #[cfg(feature = "scene")]
            chrome: chrome::Chrome::new(),
        }
    }

    fn figure(&self, source: &str) -> NodeBuilder {
        if crate::style::Metrics::current().is_grid() || crate::widgets::source_too_large(source) {
            return text_figure(source);
        }
        let bounded = bound_source(source);
        let last = self.last.borrow().clone().unwrap_or_default();
        let attempts: [(&str, Draw); 4] = [
            (&bounded, svg),
            (&bounded, vector),
            (&last, svg),
            (&last, vector),
        ];
        for (shown, draw) in attempts {
            if shown.is_empty() {
                continue;
            }
            if let Some(drawn) = draw(shown) {
                let fresh = shown == bounded;
                if fresh {
                    self.last.replace(Some(bounded.clone()));
                }
                return frame(self, drawn, &bounded, !fresh);
            }
        }
        text_figure(source)
    }
}

#[cfg(feature = "scene")]
fn frame(view: &View, drawn: Drawn, source: &str, stale: bool) -> NodeBuilder {
    view.chrome.wrap(drawn, source, stale)
}

#[cfg(not(feature = "scene"))]
fn frame(_: &View, drawn: Drawn, source: &str, _stale: bool) -> NodeBuilder {
    scrolled(drawn.node, drawn.width)
        .role(Role::Figure)
        .accessible(diagram_name(source))
        .description(source.trim_end().to_string())
}

fn svg(source: &str) -> Option<Drawn> {
    let (node, size) = picture(source)?;
    Some(Drawn {
        node,
        width: size.0,
        #[cfg(feature = "scene")]
        height: Some(size.1),
        #[cfg(feature = "scene")]
        source: source.to_string(),
    })
}

fn vector(source: &str) -> Option<Drawn> {
    let (node, width) = flow_vector::figure(source).or_else(|| sequence::vector(source))?;
    Some(Drawn {
        node,
        width,
        #[cfg(feature = "scene")]
        height: None,
        #[cfg(feature = "scene")]
        source: source.to_string(),
    })
}

fn text_figure(source: &str) -> NodeBuilder {
    let lines = draw(source);
    let extent = lines_extent(&lines);
    let col = container()
        .layout(LayoutStyle::flex_column().fill_width())
        .class("font-mono text-sm");
    let form =
        CellForm::<fn(NodeBuilder, &[String]) -> NodeBuilder>::new(strokes::window, text_lines);
    Horizontal::new(signal(extent), None)
        .wrap(form.pick()(col, &lines))
        .role(Role::Figure)
        .accessible(diagram_name(source))
        .description(bound_source(source).trim_end().to_string())
}

#[cfg(feature = "scene")]
fn picture(source: &str) -> Option<(NodeBuilder, (f32, f32))> {
    picture::figure(source)
}

#[cfg(not(feature = "scene"))]
const fn picture(_source: &str) -> Option<(NodeBuilder, (f32, f32))> {
    None
}

fn text_lines(mut col: NodeBuilder, lines: &[String]) -> NodeBuilder {
    for line in lines {
        col = col.child(text(line.clone()).nowrap());
    }
    col
}

fn diagram_name(source: &str) -> String {
    let kind = source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or("diagram");
    format!("Mermaid {kind}")
}

const MAX_OUTPUT_LINES: usize = 512;
const MAX_OUTPUT_CHARS: usize = 1024;

fn bound_output(mut lines: Vec<String>) -> Vec<String> {
    lines.truncate(MAX_OUTPUT_LINES);
    for line in &mut lines {
        if line.chars().count() > MAX_OUTPUT_CHARS {
            *line = line.chars().take(MAX_OUTPUT_CHARS).collect();
        }
    }
    lines
}

fn draw(source: &str) -> Vec<String> {
    if crate::widgets::source_too_large(source) {
        return vec![crate::widgets::oversize_message()];
    }
    let source = bound_source(source);
    let mut lines = flow::draw(&source)
        .or_else(|| sequence::draw(&source))
        .or_else(|| pie::draw(&source))
        .unwrap_or_else(|| frame_source(&source));
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    bound_output(lines)
}

#[cfg(test)]
#[path = "mermaid_tests.rs"]
mod tests;
