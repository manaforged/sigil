use core::time::Duration;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::reactive::{derived, register_interval, signal};

const FRAME_COUNT: u64 = 10;
const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
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
    register_interval(PERIOD, move || {
        tick.update(|value| *value = value.wrapping_add(1));
    });
    let frame = derived(move || spinner_frame(tick.get()).to_owned());
    container().role(Role::ProgressIndicator).child(text(frame))
}
