use std::env;
use std::fmt::Write;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::profile::{self, FRAME_BUDGET_NS, Frame};

const UNSET: u8 = 0;
const OFF: u8 = 1;
const ON: u8 = 2;

static STATE: AtomicU8 = AtomicU8::new(UNSET);
#[cfg(feature = "terminal")]
static SEEN: AtomicU8 = AtomicU8::new(UNSET);

fn state() -> u8 {
    if STATE.load(Ordering::Relaxed) == UNSET {
        set(env::var("SIGIL_HUD").is_ok_and(|value| !value.is_empty() && value != "0"));
    }
    STATE.load(Ordering::Relaxed)
}

pub(crate) fn set(on: bool) {
    STATE.store(if on { ON } else { OFF }, Ordering::Relaxed);
    if on {
        profile::enable();
    }
}

pub(crate) fn enabled() -> bool {
    state() == ON
}

#[cfg(all(test, feature = "terminal"))]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(feature = "terminal")]
pub(crate) fn take_state_change() -> bool {
    SEEN.swap(state(), Ordering::Relaxed) != state()
}

pub(crate) const fn level(wall_ns: u64) -> u8 {
    if wall_ns > FRAME_BUDGET_NS {
        2
    } else if wall_ns > FRAME_BUDGET_NS / 2 {
        1
    } else {
        0
    }
}

#[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
pub(crate) fn compact(frame: &Frame) -> String {
    let ms = |ns: u64| ns as f64 / 1_000_000.0;
    let mut line = format!(
        "f{} {} {:.1}ms",
        frame.index,
        frame.cause,
        ms(frame.wall_ns)
    );
    if let Some(backend) = frame.backend {
        let _ = write!(line, " [{backend}]");
    }
    for &(name, ns) in frame.phases.iter().take(4) {
        let _ = write!(line, " {name}={:.1}", ms(ns));
    }
    let nodes = frame.counter(crate::profile::Counter::TuiNodes)
        + frame.counter(crate::profile::Counter::SceneCmds);
    if nodes > 0 {
        let _ = write!(line, " nodes={nodes}");
    }
    line
}

#[cfg(feature = "scene")]
mod overlay {
    use super::{compact, level};
    use crate::geometry::{Point, Rect};
    use crate::profile::{self, FRAME_BUDGET_NS, Frame};
    use crate::scene::command::{DrawGlyphRun, DrawRect, SceneRectStyle, shape_glyph_run};
    use crate::scene::corner_radii::CornerRadii;
    use crate::scene::{CommandScene, SceneCommand};
    use crate::style::{Rgba8, TextStyle};

    const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
        Rgba8 { r, g, b, a }
    }

    const OK: Rgba8 = rgba(0x4c, 0xc0, 0x6a, 0xff);
    const WARN: Rgba8 = rgba(0xe0, 0xb0, 0x40, 0xff);
    const ERR: Rgba8 = rgba(0xe0, 0x50, 0x50, 0xff);
    const TEXT: Rgba8 = rgba(0xc8, 0xcd, 0xd8, 0xff);
    const BACKDROP: Rgba8 = rgba(0x08, 0x0a, 0x10, 0xc0);
    const BARS: usize = 64;
    const BAR_W: f32 = 4.0;
    const BAR_GAP: f32 = 1.0;
    const BAR_MAX_H: f32 = 26.0;
    const PAD: f32 = 8.0;

    const fn rect(x: f32, y: f32, w: f32, h: f32, style: SceneRectStyle) -> SceneCommand {
        SceneCommand::Rect(DrawRect::new(Rect::new(Point::new(x, y), w, h), style))
    }

    fn bars(scene: &mut CommandScene, shown: &[&Frame], x0: f32, y0: f32) {
        #[allow(clippy::cast_precision_loss, reason = "ns scale → px divisor")]
        let scale = FRAME_BUDGET_NS.saturating_mul(2) as f32;
        let mut x = x0 + PAD;
        for frame in shown {
            #[allow(clippy::cast_precision_loss, reason = "ns → px bar height")]
            let frac = (frame.wall_ns as f32 / scale).clamp(0.02, 1.0);
            let h = (frac * BAR_MAX_H).max(2.0);
            let color = match level(frame.wall_ns) {
                2 => ERR,
                1 => WARN,
                _ => OK,
            };
            scene.commands.push(rect(
                x,
                y0 + PAD + (BAR_MAX_H - h),
                BAR_W,
                h,
                SceneRectStyle::solid(color),
            ));
            x += BAR_W + BAR_GAP;
        }
    }

    #[allow(clippy::cast_precision_loss, reason = "≤64 bars → f32 is exact")]
    pub(crate) fn overlay(scene: &mut CommandScene, viewport: (f32, f32)) {
        let frames = profile::frames();
        if frames.is_empty() {
            return;
        }
        let shown: Vec<&Frame> = frames.iter().rev().take(BARS).rev().collect();
        let inner = (shown.len() as f32)
            .mul_add(BAR_W + BAR_GAP, -BAR_GAP)
            .max(120.0);
        let width = PAD.mul_add(2.0, inner);
        let x0 = (viewport.0 - width - 10.0).max(4.0);
        let y0 = 10.0;
        let height = PAD.mul_add(2.0, BAR_MAX_H) + 14.0;
        scene.commands.push(rect(
            x0,
            y0,
            width,
            height,
            SceneRectStyle::new(BACKDROP, CornerRadii::uniform(6.0), 10_000),
        ));
        bars(scene, &shown, x0, y0);
        if let Some(last) = frames.last() {
            let style = TextStyle {
                font_size: 10.0,
                ..TextStyle::default()
            };
            let run = shape_glyph_run(
                Rect::new(
                    Point::new(x0 + PAD, y0 + PAD + BAR_MAX_H + 2.0),
                    PAD.mul_add(-2.0, width),
                    12.0,
                ),
                compact(last),
                TEXT,
                style,
                10_000,
            );
            scene
                .commands
                .push(SceneCommand::GlyphRun(DrawGlyphRun::new(run)));
        }
    }
}

#[cfg(feature = "scene")]
pub(crate) use overlay::overlay;
