use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use core::time::Duration;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::reactive::{derived, interval, signal};
use crate::style::Metrics;

use super::shared::ICON_GLYPH;

const PERIOD: Duration = Duration::from_millis(80);

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const WINDOW_FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];

fn cycle(frames: &[&'static str], step: u64) -> &'static str {
    let len = u64::try_from(frames.len()).unwrap_or(1);
    usize::try_from(step % len)
        .ok()
        .and_then(|at| frames.get(at))
        .copied()
        .unwrap_or_default()
}

#[must_use]
pub fn spinner_frame(tick: u64) -> &'static str {
    cycle(&FRAMES, tick)
}

#[must_use]
pub fn spinner() -> NodeBuilder {
    let tick = signal(0_u64);
    interval(PERIOD, move || {
        tick.update(|value| *value = value.wrapping_add(1));
    });
    let grid = Metrics::current().is_grid();
    let indicator = container()
        .role(Role::ProgressIndicator)
        .label(theme_label(StringKey::Loading));
    #[cfg(feature = "scene")]
    if !grid {
        return indicator.children_reactive(move || {
            let degrees = tick.get() % 8 * 45;
            [super::icon_svg(format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate({degrees} 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>"
            ))]
        });
    }
    let frame = derived(move || {
        if grid {
            spinner_frame(tick.get()).to_owned()
        } else {
            cycle(&WINDOW_FRAMES, tick.get() / 2).to_owned()
        }
    });
    indicator.child(text(frame).class(ICON_GLYPH))
}

#[cfg(test)]
#[path = "spinner_tests.rs"]
mod tests;
