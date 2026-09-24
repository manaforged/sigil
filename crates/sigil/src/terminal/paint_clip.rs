use crate::app::{cell_text_lines, cell_width};
use crate::hal::Rect;
use crate::kernel::{ImageCrop, ImageSource};
use crate::scene::TextFlow;
use crate::scene::command::RunInk;
use crate::style::{TextRuns, Theme};
use unicode_segmentation::UnicodeSegmentation;

use smol_str::SmolStr;

use super::{CONTINUES, Cell, Face, Grid, Ink, Underline, decoration, floor_cell};

struct Run<'a> {
    theme: &'a Theme,
    col: usize,
    row: usize,
    last: usize,
    text: &'a str,
    fg: Ink,
    face: Face,
    link: Option<&'a SmolStr>,
    styles: Option<(&'a TextRuns, &'a [RunInk], usize)>,
    hyperlinks: bool,
}

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
    (rect, crop): (Rect, Option<ImageCrop>),
    visible: Rect,
    source: &ImageSource,
) -> Option<(u32, u32)> {
    let base = crop.map_or_else(|| (0, source.height()), |crop| (crop.top, crop.height));
    let node_bottom = rect.origin.y + rect.height;
    let visible_bottom = visible.origin.y + visible.height;
    if visible.origin.y <= rect.origin.y && visible_bottom >= node_bottom {
        return crop.map(|crop| (crop.top, crop.height));
    }
    if rect.height <= 0.0 || base.1 == 0 {
        return Some(base);
    }
    let start = ((visible.origin.y - rect.origin.y) / rect.height).clamp(0.0, 1.0);
    let end = ((visible_bottom - rect.origin.y) / rect.height).clamp(start, 1.0);
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

fn fit(text: &str, cells: usize) -> String {
    if cell_width(text) <= cells {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let step = cell_width(grapheme);
        if used + step >= cells {
            break;
        }
        used += step;
        out.push_str(grapheme);
    }
    out.push('\u{2026}');
    out
}

fn fitted(node: &Text<'_>, width: usize, room: usize) -> String {
    match node.flow {
        TextFlow::Ellipsis => fit(node.content, width),
        TextFlow::Line if room > 0 => fit(node.content, room),
        TextFlow::Line | TextFlow::Wrap => node.content.to_owned(),
    }
}

fn rows(visible: Rect, clip: Option<Rect>, cell: f32) -> (isize, isize) {
    let top = signed_cell(visible.origin.y / cell);
    let bottom = signed_cell(((visible.origin.y + visible.height) / cell).ceil()).max(top);
    let Some(clip) = clip else {
        return (top, bottom);
    };
    let clip_top = signed_cell(clip.origin.y / cell + 0.5);
    let clip_bottom = signed_cell((clip.origin.y + clip.height) / cell + 0.5).max(clip_top + 1);
    (top.max(clip_top), bottom.min(clip_bottom))
}

pub(super) struct Text<'a> {
    pub(super) rect: Rect,
    pub(super) clip: Option<Rect>,
    pub(super) content: &'a str,
    pub(super) flow: TextFlow,
    pub(super) ink: Ink,
    pub(super) face: Face,
    pub(super) link: Option<&'a SmolStr>,
    pub(super) theme: &'a Theme,
    pub(super) runs: Option<(&'a TextRuns, &'a [RunInk])>,
    pub(super) hyperlinks: bool,
}

fn centered(node: &Text<'_>, cell: f32) -> isize {
    match node.clip {
        Some(clip) if clip.height <= cell => signed_cell(clip.origin.y / cell + 0.5),
        _ => signed_cell((node.rect.origin.y + node.rect.height / 2.0) / cell),
    }
}

pub(super) fn paint_text(grid: &mut Grid, node: &Text<'_>, rect: Rect, cell: f32) {
    let col = floor_cell(rect.origin.x / cell);
    let text_width = floor_cell(node.rect.width / cell).max(1);
    let centered_line = centered(node, cell);
    let last = node.clip.map_or(usize::MAX, |clip| {
        floor_cell((clip.origin.x + clip.width) / cell)
    });
    let room =
        (node.rect.origin.x >= rect.origin.x).then(|| last.min(grid.cols()).saturating_sub(col));
    let text = fitted(node, text_width, room.unwrap_or(0));
    let lines = if node.flow == TextFlow::Wrap {
        cell_text_lines(&text, text_width)
    } else {
        vec![(0, text.len())]
    };
    let (clip_top, clip_bottom) = rows(rect, node.clip, cell);
    let top = signed_cell(node.rect.origin.y / cell);
    let single = lines.len() == 1;
    for (offset, (start, end)) in lines.into_iter().enumerate() {
        let Some(line_text) = text.get(start..end) else {
            continue;
        };
        let skipped = floor_cell((rect.origin.x - node.rect.origin.x) / cell);
        let full = line_text.len();
        let (line_text, consumed) = skip_cells(line_text, skipped);
        let base = start + full - line_text.len();
        let row = if single {
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
            theme: node.theme,
            col: col.saturating_add(consumed.saturating_sub(skipped)),
            row,
            last,
            text: line_text,
            fg: node.ink,
            face: node.face,
            link: node.link,
            styles: node.runs.map(|(runs, inks)| (runs, inks, base)),
            hyperlinks: node.hyperlinks,
        });
    }
}

impl Grid {
    fn run(&mut self, run: &Run<'_>) {
        let last = run.last.min(self.cols);
        let start = run.row.saturating_mul(self.cols);
        let Some(line) = self.cells.get_mut(start..start.saturating_add(last)) else {
            return;
        };
        let mut x = run.col;
        for (offset, text) in run.text.grapheme_indices(true) {
            let step = cell_width(text);
            if x >= last || x.saturating_add(step) > last {
                return;
            }
            if step == 0 {
                continue;
            }
            let styled = styled(run, offset);
            for pad in 0..step {
                if let Some(cell) = line.get_mut(x.saturating_add(pad)) {
                    cell.text = if pad == 0 {
                        text.into()
                    } else {
                        SmolStr::new_inline(CONTINUES)
                    };
                    paint_cell(cell, run, &styled);
                }
            }
            x = x.saturating_add(step);
        }
    }
}

struct Styled {
    face: Face,
    fg: Ink,
    bg: Option<Ink>,
    link: Option<SmolStr>,
}

fn styled(run: &Run<'_>, offset: usize) -> Styled {
    let mut out = Styled {
        face: run.face,
        fg: run.fg,
        bg: None,
        link: run.link.cloned(),
    };
    let Some((runs, inks, base)) = run.styles else {
        return out;
    };
    let byte = base + offset;
    let Some((index, text_run)) = runs
        .0
        .iter()
        .enumerate()
        .find(|(_, r)| r.start <= byte && byte < r.end)
    else {
        return out;
    };
    let face = &text_run.face;
    let ink = inks.get(index).copied().unwrap_or_default();
    out.face.bold |= face.bold;
    out.face.italic |= face.italic;
    if face.link.is_some() && !run.hyperlinks {
        out.face.underline = Underline::Straight;
    }
    out.fg = ink.color.map_or(out.fg, Ink::Exact);
    out.bg = ink.background.map(Ink::Exact);
    if let Some(link) = &face.link {
        out.link = Some(SmolStr::from(&**link));
    }
    out
}

fn paint_cell(cell: &mut Cell, run: &Run<'_>, styled: &Styled) {
    cell.bold = styled.face.bold;
    cell.italic = styled.face.italic;
    cell.dim = styled.face.dim;
    cell.link.clone_from(&styled.link);
    if styled.face.underline != Underline::None {
        cell.lines.underline = styled.face.underline;
    }
    if let Some(bg) = styled.bg {
        cell.bg = bg;
    }
    if styled.fg.set() {
        cell.fg = decoration::over(cell.bg, styled.fg, run.theme);
    }
}
