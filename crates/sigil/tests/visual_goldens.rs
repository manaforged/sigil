#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::prelude::*;

use sigil_kittest::{
    InteractionMode, assert_goldens_differ, assert_rendered_matches_golden, golden_path,
    render_widget_to_png_focus_descendant, render_widget_to_png_stub,
};

const DELTA_PCT: f32 = 0.5;

fn on_panel(inner: impl IntoNode + 'static, w: f32, h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(w, h)
                .padding(6.0)
                .align_items(AlignItems::Stretch),
        )
        .child(inner)
}

#[path = "visual_goldens/controls.rs"]
mod controls;
#[path = "visual_goldens/fields.rs"]
mod fields;
#[path = "visual_goldens/overlays.rs"]
mod overlays;
#[path = "visual_goldens/table.rs"]
mod table;
