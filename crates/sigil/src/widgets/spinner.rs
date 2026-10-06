use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use core::time::Duration;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::reactive::{derived, interval, signal};
use crate::style::Metrics;

use super::shared::ICON_GLYPH;

const FRAME_COUNT: u64 = 10;
const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const WINDOW_FRAME_COUNT: u64 = 4;
const WINDOW_FRAMES: [&str; 4] = ["◐", "◓", "◑", "◒"];
const PERIOD: Duration = Duration::from_millis(80);
#[cfg(feature = "scene")]
const ARC_FRAMES: [&str; 8] = [
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate(0 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate(45 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate(90 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate(135 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate(180 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate(225 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate(270 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>",
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.5\" stroke-linecap=\"round\"><path transform=\"rotate(315 12 12)\" d=\"M21 12a9 9 0 1 1-6.219-8.56\"/></svg>",
];

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
    let indicator = container()
        .role(Role::ProgressIndicator)
        .label(theme_label(StringKey::Loading));
    #[cfg(feature = "scene")]
    if !grid {
        return indicator.children_reactive(move || {
            let at = usize::try_from(tick.get() % ARC_FRAMES.len() as u64).unwrap_or(0);
            [super::icon_svg(
                ARC_FRAMES.get(at).copied().unwrap_or_default(),
            )]
        });
    }
    let frame = derived(move || {
        if grid {
            spinner_frame(tick.get()).to_owned()
        } else {
            window_frame(tick.get()).to_owned()
        }
    });
    indicator.child(text(frame).class(ICON_GLYPH))
}

fn window_frame(tick: u64) -> &'static str {
    usize::try_from(tick / 2 % WINDOW_FRAME_COUNT)
        .ok()
        .and_then(|at| WINDOW_FRAMES.get(at))
        .copied()
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "spinner_tests.rs"]
mod tests;
