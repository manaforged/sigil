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

pub(crate) fn figure(source: &str, fallback: Fallback) -> NodeBuilder {
    #[cfg(vega_js)]
    if let Some(node) = view::figure(source, fallback) {
        return node;
    }
    fallback(source)
}
