use super::*;

impl CellWalk<'_> {
    pub(super) fn image(&mut self, draw: &DrawImage) {
        let destination = self.fitted(draw);
        if self.grid.graphics && draw.crop.is_none() && self.place_full(destination, &draw.source) {
            return;
        }
        let Some((visible, origin, size)) = self.area(destination) else {
            return;
        };
        let crop = image_crop((self.map(destination), draw.crop), visible, &draw.source);
        self.grid.place(&draw.source, crop, origin, size);
    }

    fn place_full(&mut self, destination: Rect, source: &crate::kernel::ImageSource) -> bool {
        let mapped = self.map(destination);
        let (full_col, full_width) =
            signed_span(mapped.origin.x / self.cell, mapped.width / self.cell);
        let (full_row, full_height) =
            signed_span(mapped.origin.y / self.cell, mapped.height / self.cell);
        if full_width == 0 || full_height == 0 || full_width > 256 || full_height > 256 {
            return false;
        }
        let cols = i32::try_from(self.grid.cols()).unwrap_or(i32::MAX);
        let rows = i32::try_from(self.grid.rows()).unwrap_or(i32::MAX);
        let left = full_col.max(0);
        let top = full_row.max(0);
        let right = (full_col + i32::from(full_width)).min(cols);
        let bottom = (full_row + i32::from(full_height)).min(rows);
        if left >= right || top >= bottom {
            return true;
        }
        let mark_col = u16::try_from(left - full_col).unwrap_or(0);
        let mark_row = u16::try_from(top - full_row).unwrap_or(0);
        let visible_cols = u16::try_from(right - left).unwrap_or(0);
        let visible_rows = u16::try_from(bottom - top).unwrap_or(0);
        self.grid.images.push(crate::terminal::paint::Placement {
            col: u16::try_from(left).unwrap_or(0),
            row: u16::try_from(top).unwrap_or(0),
            cols: visible_cols,
            rows: visible_rows,
            mark_col,
            mark_row,
            span_cols: full_width,
            span_rows: full_height,
            source: source.clone(),
            crop: None,
        });
        true
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "image dimensions are far below the f32 mantissa limit"
    )]
    fn fitted(&self, draw: &DrawImage) -> Rect {
        if draw.fit != ObjectFit::Contain {
            return draw.destination;
        }
        let height = draw.source.height();
        let rows = draw
            .crop
            .and_then(|crop| crop.rows(height))
            .map_or(height, |(_, rows)| rows);
        let shown = Rect::new(
            Point::new(0.0, 0.0),
            draw.source.width() as f32,
            rows as f32 * self.paint.caps.cell_aspect(),
        );
        fit(ObjectFit::Contain, shown, draw.destination).1
    }
}
