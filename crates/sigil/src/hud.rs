use std::env;
use std::fmt::Write;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::profile::{self, Frame};

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

pub(crate) fn level(work_ns: u64) -> u8 {
    let budget = profile::budget_ns();
    if work_ns > budget {
        2
    } else if work_ns > budget / 2 {
        1
    } else {
        0
    }
}

#[allow(clippy::cast_precision_loss, reason = "ns → ms display only")]
pub(crate) fn compact(frame: &Frame) -> String {
    let ms = |ns: u64| ns as f64 / 1_000_000.0;
    let work = frame.work_ns();
    let mut line = format!(
        "f{} {} {:.1}ms wait {:.1}",
        frame.index,
        frame.cause,
        ms(work),
        ms(frame.total_ns().saturating_sub(work))
    );
    if let Some(backend) = frame.backend {
        let _ = write!(line, " [{backend}]");
    }
    if let Some((name, ns)) = frame.hottest() {
        let _ = write!(line, " {name}={:.1}", ms(ns));
    }
    if let Some(site) = frame.sites.first() {
        let name = crate::profile::site_name(site.location);
        let short = name.rsplitn(3, '/').take(2).collect::<Vec<_>>();
        let _ = write!(
            line,
            " {}/{}={:.1}",
            short.get(1).unwrap_or(&""),
            short[0],
            ms(site.self_ns)
        );
    }
    if frame.overhead_ns > 0 {
        let _ = write!(line, " prof={:.2}", ms(frame.overhead_ns));
    }
    let nodes = frame.counter(crate::profile::Counter::TuiNodes)
        + frame.counter(crate::profile::Counter::SceneCmds);
    if nodes > 0 {
        let _ = write!(line, " nodes={nodes}");
    }
    line
}

#[cfg(feature = "desktop")]
mod overlay {
    use super::{compact, level};
    use crate::geometry::{Point, Rect};
    use crate::profile::{self, Frame};
    use std::cell::RefCell;

    use crate::scene::command::{DrawRect, GlyphRun, shape_glyph_run};
    use crate::scene::corner_radii::CornerRadii;
    use crate::scene::{CommandScene, SceneCommand};
    use crate::style::{Rgba8, TextStyle};

    const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
        Rgba8 { r, g, b, a }
    }

    const OK: Rgba8 = rgba(0x4c, 0xc0, 0x6a, 0xff);
    const WARN: Rgba8 = rgba(0xe0, 0xb0, 0x40, 0xff);
    const ERR: Rgba8 = rgba(0xe0, 0x50, 0x50, 0xff);
    const TEXT_INK: Rgba8 = rgba(0xc8, 0xcd, 0xd8, 0xff);
    const BACKDROP: Rgba8 = rgba(0x08, 0x0a, 0x10, 0xc0);
    const BARS: usize = 64;
    const BAR_W: f32 = 4.0;
    const BAR_GAP: f32 = 1.0;
    const BAR_MAX_H: f32 = 26.0;
    const PAD: f32 = 8.0;
    const TEXT_ADVANCE: f32 = 6.0;

    const fn at(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(Point::new(x, y), w, h)
    }

    fn bars(scene: &mut CommandScene, works: &[u64], x0: f32, y0: f32) {
        #[allow(clippy::cast_precision_loss, reason = "ns scale → px divisor")]
        let scale = profile::budget_ns().saturating_mul(2) as f32;
        let mut x = x0 + PAD;
        for &work in works {
            #[allow(clippy::cast_precision_loss, reason = "ns → px bar height")]
            let frac = (work as f32 / scale).clamp(0.02, 1.0);
            let h = (frac * BAR_MAX_H).max(2.0);
            let color = match level(work) {
                2 => ERR,
                1 => WARN,
                _ => OK,
            };
            scene.commands.push(SceneCommand::Rect(DrawRect::new(
                at(x, y0 + PAD + (BAR_MAX_H - h), BAR_W, h),
                color,
            )));
            x += BAR_W + BAR_GAP;
        }
    }

    struct Text {
        frame: u32,
        viewport: u32,
        width: f32,
        run: GlyphRun,
    }

    thread_local! {
        static TEXT: RefCell<Option<Text>> = const { RefCell::new(None) };
    }

    #[allow(clippy::cast_precision_loss, reason = "≤64 bars → f32 is exact")]
    fn text(worst: &Frame, viewport: (f32, f32), bars: usize) -> (f32, GlyphRun) {
        TEXT.with(|cell| {
            let mut cached = cell.borrow_mut();
            if let Some(text) = cached.as_ref()
                && text.frame == worst.index
                && text.viewport == viewport.0.to_bits()
            {
                return (text.width, text.run.clone());
            }
            let line = compact(worst);
            let inner = (bars as f32)
                .mul_add(BAR_W + BAR_GAP, -BAR_GAP)
                .max(120.0)
                .max(line.chars().count() as f32 * TEXT_ADVANCE)
                .min(viewport.0 - 40.0);
            let width = PAD.mul_add(2.0, inner);
            let (x0, y0) = origin(viewport, width);
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
                line,
                TEXT_INK,
                style,
                10_000,
            );
            *cached = Some(Text {
                frame: worst.index,
                viewport: viewport.0.to_bits(),
                width,
                run: run.clone(),
            });
            (width, run)
        })
    }

    fn origin(viewport: (f32, f32), width: f32) -> (f32, f32) {
        ((viewport.0 - width - 10.0).max(4.0), 10.0)
    }

    pub(crate) fn overlay(scene: &mut CommandScene, viewport: (f32, f32)) {
        let mut works = [0_u64; BARS];
        let Some((count, width, run)) = profile::with_frames(|frames| {
            let start = frames.len().saturating_sub(BARS);
            let mut worst: Option<&Frame> = None;
            for (slot, frame) in works.iter_mut().zip(frames.range(start..)) {
                *slot = frame.work_ns();
                if worst.is_none_or(|best| best.work_ns() < *slot) {
                    worst = Some(frame);
                }
            }
            let worst = worst?;
            let (width, run) = text(worst, viewport, frames.len() - start);
            Some((frames.len() - start, width, run))
        }) else {
            return;
        };
        let (x0, y0) = origin(viewport, width);
        let height = PAD.mul_add(2.0, BAR_MAX_H) + 14.0;
        scene.commands.push(SceneCommand::Rect(
            DrawRect::new(at(x0, y0, width, height), BACKDROP)
                .border_radius(CornerRadii::uniform(6.0)),
        ));
        bars(scene, &works[..count], x0, y0);
        scene.commands.push(SceneCommand::GlyphRun(run));
    }
}

#[cfg(feature = "desktop")]
pub(crate) use overlay::overlay;
