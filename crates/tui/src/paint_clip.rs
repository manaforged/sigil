use sigil::app::{Painted, cell_text_lines, cell_width};
use sigil::hal::Rect;
use sigil::kernel::ImageSource;
use unicode_segmentation::UnicodeSegmentation;

use super::{Grid, Ink, Run, floor_cell};

pub(super) fn skip_cells(text: &str, cells: usize) -> (&str, usize) {
    let mut used = 0;
    for (start, grapheme) in text.grapheme_indices(true) {
        if used >= cells {
            return (&text[start..], used);
        }
        used = used.saturating_add(cell_width(grapheme));
    }
    ("", used)
}

pub(super) fn image_crop(
    node: &Painted,
    visible: Rect,
    source: &ImageSource,
) -> Option<(u32, u32)> {
    let base = node
        .crop
        .map_or_else(|| (0, source.height()), |crop| (crop.top, crop.height));
    let node_bottom = node.rect.origin.y + node.rect.height;
    let visible_bottom = visible.origin.y + visible.height;
    if visible.origin.y <= node.rect.origin.y && visible_bottom >= node_bottom {
        return node.crop.map(|crop| (crop.top, crop.height));
    }
    if node.rect.height <= 0.0 || base.1 == 0 {
        return Some(base);
    }
    let start = ((visible.origin.y - node.rect.origin.y) / node.rect.height).clamp(0.0, 1.0);
    let end = ((visible_bottom - node.rect.origin.y) / node.rect.height).clamp(start, 1.0);
    let top = crop_row(start, base.1, false);
    let bottom = crop_row(end, base.1, true);
    Some((
        base.0.saturating_add(top),
        bottom.saturating_sub(top).max(1),
    ))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "source rows are bounded by the image crop height"
)]
fn crop_row(fraction: f32, height: u32, round_up: bool) -> u32 {
    let scaled = fraction * height as f32;
    let row = if round_up {
        scaled.ceil()
    } else {
        scaled.floor()
    };
    row.clamp(0.0, height as f32) as u32
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the signed terminal range"
)]
pub(super) const fn signed_cell(value: f32) -> isize {
    value.floor() as isize
}

pub(super) fn paint_text(grid: &mut Grid, node: &Painted, rect: Rect, cell: f32) {
    let Some(text) = node.text.as_deref() else {
        return;
    };
    let col = floor_cell(rect.origin.x / cell);
    let text_width = floor_cell(node.rect.width / cell).max(1);
    let ink = Ink::pick(node.ink, node.tone);
    let centered_line = match node.clip {
        Some(clip) if clip.height <= cell => signed_cell(clip.origin.y / cell + 0.5),
        _ => signed_cell((node.rect.origin.y + node.rect.height / 2.0) / cell),
    };
    let last = node.clip.map_or(usize::MAX, |clip| {
        floor_cell((clip.origin.x + clip.width) / cell)
    });
    let lines = if node.wrap {
        cell_text_lines(text, text_width)
    } else {
        vec![(0, text.len())]
    };
    let clip_top = signed_cell(rect.origin.y / cell);
    let clip_bottom = signed_cell(((rect.origin.y + rect.height) / cell).ceil()).max(clip_top);
    let top = signed_cell(node.rect.origin.y / cell);
    for (offset, (start, end)) in lines.into_iter().enumerate() {
        let Some(line_text) = text.get(start..end) else {
            continue;
        };
        let skipped = floor_cell((rect.origin.x - node.rect.origin.x) / cell);
        let (line_text, consumed) = skip_cells(line_text, skipped);
        let row = if !node.wrap && offset == 0 {
            centered_line
        } else {
            top.saturating_add(isize::try_from(offset).unwrap_or(isize::MAX))
        };
        if row < clip_top || row >= clip_bottom || row < 0 {
            continue;
        }
        let Ok(row) = usize::try_from(row) else {
            continue;
        };
        grid.run(&Run {
            col: col.saturating_add(consumed.saturating_sub(skipped)),
            row,
            last,
            text: line_text,
            fg: ink,
            bold: node.weight >= 700,
            italic: node.italic,
        });
    }
}
