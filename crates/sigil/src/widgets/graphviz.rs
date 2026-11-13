use crate::kernel::{NodeBuilder, Role, container, fragment, text};
use crate::layout::LayoutStyle;
use crate::reactive::{Reactive, signal};
use crate::widgets::Layout;
use crate::widgets::scroll::Horizontal;
use crate::widgets::{bound_source, frame_source, lines_extent, scrolled_diagram};

#[cfg(feature = "scene")]
mod bounds;
#[cfg(feature = "scene")]
mod svg;

/// A rendered Graphviz DOT graph.
pub fn graphviz(source: impl Reactive<String>) -> NodeBuilder {
    fragment().children_reactive(move || [figure(&source.read())])
}

pub(crate) fn figure(source: &str) -> NodeBuilder {
    if crate::widgets::source_too_large(source) {
        return framed_figure(source, &[crate::widgets::oversize_message()]);
    }
    let bounded = bound_source(source);
    if !crate::style::Metrics::current().is_grid()
        && let Some((diagram, width)) = picture(&bounded)
    {
        return scrolled_diagram(diagram, width)
            .role(Role::Figure)
            .label(graph_name(source))
            .description(bounded.trim_end().to_string());
    }
    framed_figure(source, &frame_source(&bounded))
}

fn framed_figure(source: &str, lines: &[String]) -> NodeBuilder {
    let extent = lines_extent(lines);
    let mut col = container()
        .layout(LayoutStyle::flex_column().fill_width())
        .class("font-mono text-sm");
    for line in lines {
        col = col.child(text(line.clone()).nowrap());
    }
    Horizontal::new(signal(extent), None)
        .wrap(col)
        .role(Role::Figure)
        .label(graph_name(source))
        .description(bound_source(source).trim_end().to_string())
}

#[cfg(feature = "scene")]
fn picture(source: &str) -> Option<(NodeBuilder, f32)> {
    svg::figure(source)
}

#[cfg(not(feature = "scene"))]
const fn picture(_source: &str) -> Option<(NodeBuilder, f32)> {
    None
}

fn graph_name(source: &str) -> String {
    let kind = source
        .split_whitespace()
        .find(|word| !word.eq_ignore_ascii_case("strict"))
        .map(|word| word.split('{').next().unwrap_or(word))
        .filter(|word| !word.is_empty())
        .unwrap_or("graph");
    format!("Graphviz {kind}")
}
