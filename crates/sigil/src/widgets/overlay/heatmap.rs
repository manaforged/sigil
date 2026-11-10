use crate::kernel::{NodeBuilder, ObjectFit, Role, container, image};
use crate::layout::LayoutStyle;
use crate::style::Color;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_label};
use crate::widgets::Widget;

mod raster;

/// A grid of colored cells, drawn as one image that fills its box.
#[must_use]
pub fn heatmap<F: Fn(usize, usize) -> Color>(
    grid_w: usize,
    grid_h: usize,
    color: F,
) -> NodeBuilder {
    let root = container()
        .role(Role::Figure)
        .label(theme_label(StringKey::Heatmap))
        .description(ActiveTheme::capture().fill(StringKey::HeatmapGrid, &[&grid_w, &grid_h]))
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0),
        );
    let Some(grid) = raster::HeatGrid::new(grid_w, grid_h, &color) else {
        return root;
    };
    let Some(source) = grid.source() else {
        return root;
    };
    let (min_w, min_h) = grid.min_size();
    let themed = grid.themed(source.clone());
    let picture = image(source)
        .fit(ObjectFit::Fill)
        .mutate(move |node| {
            let _ = node.attach(themed);
        })
        .layout(
            LayoutStyle::block()
                .fill_width()
                .grow(1.0)
                .min_size(min_w, min_h),
        );
    root.child(picture)
}

#[cfg(test)]
#[path = "heatmap_tests.rs"]
mod tests;
