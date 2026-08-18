//! Screen-space debug overlay widgets.
//!
//! These widgets are intentionally simple paint-only primitives that
//! compose existing Sigil layout + style atoms. They exist for diagnostic
//! visualisations — heatmaps, hit-box overlays, debug grids — and are not
//! intended for production UI.

pub mod heatmap;

pub use heatmap::heatmap;
