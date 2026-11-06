use crate::hud;
use crate::profile::{self, Frame};
use crate::style::Tone;
use crate::terminal::paint::{Grid, Ink};

fn tone(work_ns: u64) -> Tone {
    match hud::level(work_ns) {
        2 => Tone::Error,
        1 => Tone::Warn,
        _ => Tone::Success,
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "frame times fit in f32 at glyph precision"
)]
fn spark(work_ns: u64) -> char {
    let scale = profile::budget_ns().saturating_mul(2);
    crate::widgets::spark_glyph(work_ns as f32 / scale.max(1) as f32)
}

pub(crate) fn draw(grid: &mut Grid) {
    let cols = grid.cols();
    if cols < 16 || grid.rows() < 4 {
        return;
    }
    let width = cols.saturating_sub(4).min(64);
    let col = cols.saturating_sub(width + 1);
    profile::with_frames(|frames| {
        let start = frames.len().saturating_sub(width);
        let mut worst: Option<&Frame> = None;
        let mut glyph = [0_u8; 4];
        for (i, frame) in frames.range(start..).enumerate() {
            let work = frame.work_ns();
            grid.overlay_text(
                col + i,
                0,
                spark(work).encode_utf8(&mut glyph),
                Ink::Of(tone(work)),
            );
            if worst.is_none_or(|best| best.work_ns() < work) {
                worst = Some(frame);
            }
        }
        if let Some(worst) = worst {
            let text: String = hud::compact(worst).chars().take(width).collect();
            grid.overlay_text(col, 1, &format!("{text:width$}"), Ink::Of(Tone::Neutral));
        }
    });
}

#[cfg(test)]
#[path = "hud_tests.rs"]
mod tests;
