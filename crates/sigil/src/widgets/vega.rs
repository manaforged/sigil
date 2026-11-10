use crate::kernel::NodeBuilder;

#[cfg(all(feature = "vega", not(target_arch = "wasm32")))]
mod engine;
#[cfg(all(feature = "vega", not(target_arch = "wasm32")))]
mod measure;
#[cfg(all(feature = "vega", not(target_arch = "wasm32")))]
mod script;
#[cfg(all(feature = "vega", not(target_arch = "wasm32")))]
mod theme;
#[cfg(all(feature = "vega", not(target_arch = "wasm32")))]
mod view;

pub(crate) type Fallback = fn(&str) -> NodeBuilder;

pub(crate) fn figure(source: &str, fallback: Fallback) -> NodeBuilder {
    #[cfg(all(feature = "vega", not(target_arch = "wasm32")))]
    if let Some(node) = view::figure(source, fallback) {
        return node;
    }
    fallback(source)
}
