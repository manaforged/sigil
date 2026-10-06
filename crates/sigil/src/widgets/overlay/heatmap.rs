use crate::kernel::{NodeBuilder, ObjectFit, Role, container, image};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::style::Rgba8;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_label};

use super::super::layout_ext::WidgetLayoutExt;

mod raster;

#[must_use]
pub fn heatmap<F: Fn(usize, usize) -> Rgba8>(
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
    let Some(grid) = raster::Grid::new(grid_w, grid_h, &color) else {
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
