#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::app::DefaultTextMeasure;

use sigil::prelude::*;

use sigil_kittest::{
    InteractionMode, assert_rendered_matches_golden, golden_path, render_widget_to_png_stub,
    render_widget_to_png_with,
};

const DELTA_PCT: f32 = 0.5;

fn on_black(inner: impl IntoNode + 'static, w: f32, h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(w, h)
                .padding(6.0)
                .align_items(AlignItems::Stretch),
        )
        .child(inner)
}

#[path = "visual_goldens_extra/controls.rs"]
mod controls;
#[path = "visual_goldens_extra/drawing.rs"]
mod drawing;
#[path = "visual_goldens_extra/layouts.rs"]
mod layouts;
#[path = "visual_goldens_extra/lists.rs"]
mod lists;
