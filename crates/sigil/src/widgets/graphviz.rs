use crate::kernel::{NodeBuilder, Role, fragment};
use crate::reactive::Reactive;
use crate::widgets::{bound_source, frame_source, framed_figure, scrolled_diagram};

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
        return framed_figure(
            source,
            graph_name(source),
            &[crate::widgets::oversize_message()],
        );
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
    framed_figure(source, graph_name(source), &frame_source(&bounded))
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
