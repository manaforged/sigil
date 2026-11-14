use crate::kernel::NodeBuilder;

#[cfg(vega_js)]
mod engine;
#[cfg(vega_js)]
mod measure;
#[cfg(vega_js)]
mod script;
#[cfg(vega_js)]
mod theme;
#[cfg(vega_js)]
mod view;

pub(in crate::widgets) const PLOT_HEIGHT: f32 = 300.0;
pub(in crate::widgets) const MAX_HEIGHT: f32 = 1200.0;

pub(crate) type Fallback = fn(&str) -> NodeBuilder;

/// A rendered Vega or Vega-Lite specification.
pub fn vega(source: impl crate::reactive::Reactive<String>) -> NodeBuilder {
    crate::kernel::fragment().children_reactive(move || [figure(&source.read(), framed_fallback)])
}

pub(crate) fn figure(source: &str, fallback: Fallback) -> NodeBuilder {
    #[cfg(vega_js)]
    if let Some(node) = view::figure(source, fallback) {
        return node;
    }
    fallback(source)
}

fn framed_fallback(source: &str) -> NodeBuilder {
    if crate::widgets::source_too_large(source) {
        return crate::widgets::framed_figure(
            source,
            "Vega chart",
            &[crate::widgets::oversize_message()],
        );
    }
    let bounded = crate::widgets::bound_source(source);
    crate::widgets::framed_figure(
        source,
        "Vega chart",
        &crate::widgets::frame_source(&bounded),
    )
}
