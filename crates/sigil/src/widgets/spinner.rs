use core::time::Duration;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::reactive::{derived, interval, signal};
use crate::style::{Metrics, NodeStyleExt};

use super::shared::ICON_GLYPH;

const FRAME_COUNT: u64 = 10;
const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const WINDOW_FRAME_COUNT: u64 = 4;
const WINDOW_FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];
const PERIOD: Duration = Duration::from_millis(80);

#[must_use]
pub fn spinner_frame(tick: u64) -> &'static str {
    usize::try_from(tick % FRAME_COUNT)
        .ok()
        .and_then(|at| FRAMES.get(at))
        .copied()
        .unwrap_or_default()
}

#[must_use]
pub fn spinner() -> NodeBuilder {
    let tick = signal(0_u64);
    interval(PERIOD, move || {
        tick.update(|value| *value = value.wrapping_add(1));
    });
    let grid = Metrics::current().is_grid();
    let frame = derived(move || {
        if grid {
            spinner_frame(tick.get()).to_owned()
        } else {
            window_frame(tick.get()).to_owned()
        }
    });
    container()
        .role(Role::ProgressIndicator)
        .label("Loading")
        .child(text(frame).class(ICON_GLYPH))
}

fn window_frame(tick: u64) -> &'static str {
    usize::try_from(tick / 2 % WINDOW_FRAME_COUNT)
        .ok()
        .and_then(|at| WINDOW_FRAMES.get(at))
        .copied()
        .unwrap_or_default()
}
