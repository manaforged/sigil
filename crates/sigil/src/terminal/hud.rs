use crate::hud;
use crate::profile::{self, Frame};
use crate::style::Tone;
use crate::terminal::paint::{Grid, Ink};

const SPARKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

const fn tone(wall_ns: u64) -> Tone {
    match hud::level(wall_ns) {
        2 => Tone::Error,
        1 => Tone::Warn,
        _ => Tone::Success,
    }
}

fn spark(wall_ns: u64) -> char {
    let scale = crate::profile::FRAME_BUDGET_NS.saturating_mul(2);
    let level = wall_ns
        .saturating_mul(7)
        .checked_div(scale)
        .unwrap_or(7)
        .min(7);
    SPARKS
        .get(usize::try_from(level).unwrap_or(0))
        .copied()
        .unwrap_or('▁')
}

pub(crate) fn draw(grid: &mut Grid) {
    let frames = profile::frames();
    if frames.is_empty() {
        return;
    }
    let cols = grid.cols();
    if cols < 16 || grid.rows() < 4 {
        return;
    }
    let width = cols.saturating_sub(4).min(64);
    let col = cols.saturating_sub(width + 1);
    let shown: Vec<&Frame> = frames.iter().rev().take(width).rev().collect();
    for (i, frame) in shown.iter().enumerate() {
        let glyph = spark(frame.wall_ns).to_string();
        grid.overlay_text(col + i, 0, &glyph, Ink::Of(tone(frame.wall_ns)));
    }
    let Some(last) = frames.last() else {
        return;
    };
    let text: String = hud::compact(last).chars().take(width).collect();
    grid.overlay_text(col, 1, &format!("{text:width$}"), Ink::Of(Tone::Neutral));
}

#[cfg(test)]
#[path = "hud_tests.rs"]
mod tests;
