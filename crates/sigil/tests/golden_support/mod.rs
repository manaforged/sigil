//! Visual-golden PNG harness for sigil widgets.
//!
//! Public API mirrors the brief: drive a widget into one of five interaction
//! states, render through the wgpu offscreen path, then assert against a
//! committed PNG byte-for-byte (with a small pixel-delta budget). Setting
//! `SIGIL_REGEN_GOLDENS=1` rewrites the file instead of asserting.
//!
//! `InteractionState` is intentionally NOT an external lever in sigil — each
//! widget owns its own [`crate::interaction::InteractionState`] reactive
//! bundle, driven by `EventRouter` pointer/key events. So the harness instead
//! takes an [`InteractionMode`] enum and applies it via the same event-router
//! pathway the existing `widget_states.rs` integration tests use.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::print_stdout, clippy::doc_markdown)]
#![allow(clippy::too_many_lines, clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments, clippy::missing_const_for_fn)]
#![allow(unreachable_pub, dead_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use pollster::FutureExt;
use sigil::prelude::*;
use sigil_hal::{
    Color, ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind,
    PointerPhase, ShellEvent,
};
use sigil_renderer::{CommandScene, OffscreenTarget, WgpuRenderer, emit_scene};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

/// Safety cap on `*AfterFrames { frames: u32::MAX, .. }` ticking. 600
/// frames at 60 Hz = 10 s of simulated time — well past any production
/// transition. A runaway animator that never settles will trip this and
/// panic with a clear message rather than stall the test silently.
pub const SETTLE_SAFETY_CAP: u32 = 600;

/// Which interaction state to drive the widget into before capture.
///
/// The `*AfterFrames` variants drive the same event the non-AfterFrames
/// variant does, then advance `sigil_animation_runtime::tick(dt)` for
/// `frames` iterations so animator-driven style atoms settle to the
/// post-tick value before `emit_scene` captures.
///
/// `frames: u32::MAX` is a sentinel meaning "tick until every active
/// animator reports settled, OR until [`SETTLE_SAFETY_CAP`] is hit." A
/// runaway animator that never settles will panic with the cap value in
/// the message so the test surfaces the failure rather than stalling.
///
/// `dt` is the per-tick delta in seconds; `1.0 / 60.0` is the canonical
/// 60 Hz value.
#[derive(Clone, Copy, Debug)]
pub enum InteractionMode {
    /// Resting / neutral.
    Default,
    /// Pointer is over the widget but no button is down.
    Hover,
    /// Pointer is down inside the widget (active press).
    Pressed,
    /// Full click — pointer down then up — for widgets that act on
    /// release (dropdown open, toggle flip).
    Clicked,
    /// Keyboard focus on the widget.
    Focused,
    /// Widget disabled (the widget builder must own this — harness skips events).
    Disabled,
    /// Pointer-enter, then tick the animation runtime so hover-driven
    /// transitions resolve to their post-tick value.
    HoverAfterFrames {
        /// Number of `tick(dt)` calls. `u32::MAX` ticks until settled
        /// (capped by [`SETTLE_SAFETY_CAP`]).
        frames: u32,
        /// Per-tick delta in seconds.
        dt: f32,
    },
    /// Pointer-down, then tick the animation runtime so press-driven
    /// transitions resolve to their post-tick value.
    PressedAfterFrames {
        /// Number of `tick(dt)` calls. `u32::MAX` ticks until settled
        /// (capped by [`SETTLE_SAFETY_CAP`]).
        frames: u32,
        /// Per-tick delta in seconds.
        dt: f32,
    },
    /// Focus the widget, then tick the animation runtime so focus-driven
    /// transitions resolve to their post-tick value.
    FocusedAfterFrames {
        /// Number of `tick(dt)` calls. `u32::MAX` ticks until settled
        /// (capped by [`SETTLE_SAFETY_CAP`]).
        frames: u32,
        /// Per-tick delta in seconds.
        dt: f32,
    },
}

impl InteractionMode {
    /// Lowercase tag used in golden filenames. AfterFrames variants
    /// share the tag of their non-AfterFrames counterpart so animated
    /// goldens can opt-in to a stable name with a `_animated` suffix at
    /// the call site.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Hover | Self::HoverAfterFrames { .. } => "hover",
            Self::Pressed | Self::PressedAfterFrames { .. } => "pressed",
            Self::Clicked => "clicked",
            Self::Focused | Self::FocusedAfterFrames { .. } => "focused",
            Self::Disabled => "disabled",
        }
    }
}

/// 4-pixel-wide black separator between frames in a filmstrip.
pub const FILMSTRIP_GAP_PX: u32 = 4;

/// Render an animation filmstrip: a single wide RgbaImage containing
/// `frame_count` frames stitched horizontally with a [`FILMSTRIP_GAP_PX`]
/// black separator between each frame.
///
/// Frame 0 renders the widget in the default state. Frames 1..N render
/// after applying hover (pointer enter at `hit_point`) and advancing
/// `sigil_animation_runtime::tick(dt)` once per frame. The point of the
/// filmstrip is to show the *full transition curve* at a glance — frame
/// 0 = default, frame 1 = hover-engaged, frames 2..N = interpolating.
///
/// `build` is called once per frame so each frame gets a fresh widget
/// tree under its own [`Runtime`] scope; animator state lives in a
/// per-thread cell, so ticking across frames is the canonical way to
/// observe the animation curve.
///
/// Returns `None` when no wgpu adapter is available on the host.
#[must_use]
pub fn capture_filmstrip(
    build: impl Fn(&mut Cx) -> NodeId,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    frame_count: u32,
    dt: f32,
) -> Option<image::RgbaImage> {
    assert!(
        frame_count >= 1,
        "capture_filmstrip: frame_count must be at least 1",
    );
    let mut frames: Vec<image::RgbaImage> = Vec::with_capacity(frame_count as usize);
    for index in 0..frame_count {
        let mode = if index == 0 {
            InteractionMode::Default
        } else {
            // After-frames count = ticks since hover engaged. Frame 1
            // is "hover just applied, zero ticks" — this matches the
            // brief's contract that frame 1 looks the same as frame 0
            // (transition just started) or slightly transitioning.
            InteractionMode::HoverAfterFrames {
                frames: index - 1,
                dt,
            }
        };
        let frame = render_widget_to_png(&build, mode, size_px, hit_point)?;
        frames.push(frame);
    }
    Some(stitch_horizontal(&frames, FILMSTRIP_GAP_PX))
}

/// Stitch `frames` horizontally with a `gap_px` black separator between
/// each pair. Frames must all share the same height; the result width is
/// `n * frame_width + (n - 1) * gap_px`.
fn stitch_horizontal(frames: &[image::RgbaImage], gap_px: u32) -> image::RgbaImage {
    assert!(!frames.is_empty(), "stitch_horizontal: no frames");
    let frame_w = frames[0].width();
    let height = frames[0].height();
    for frame in frames {
        assert_eq!(frame.width(), frame_w, "filmstrip frame width mismatch");
        assert_eq!(frame.height(), height, "filmstrip frame height mismatch");
    }
    let n = frames.len() as u32;
    let total_w = n * frame_w + n.saturating_sub(1) * gap_px;
    let mut out = image::RgbaImage::from_pixel(total_w, height, image::Rgba([0, 0, 0, 255]));
    for (index, frame) in frames.iter().enumerate() {
        let x_offset = index as u32 * (frame_w + gap_px);
        for y in 0..height {
            for x in 0..frame_w {
                let pixel = *frame.get_pixel(x, y);
                out.put_pixel(x_offset + x, y, pixel);
            }
        }
    }
    out
}

/// Minimal text measurement stub matching the existing widget tests.
struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

/// Return `true` when `SIGIL_REGEN_GOLDENS=1` is set in the environment.
#[must_use]
pub fn regen_enabled() -> bool {
    matches!(
        std::env::var("SIGIL_REGEN_GOLDENS").ok().as_deref(),
        Some("1" | "true" | "yes" | "on"),
    )
}

/// Return `true` when `SIGIL_REAL_MEASURE=1` is set. Diagnostic switch:
/// the `StubMeasure`-backed `render_widget_to_png` then measures text
/// through the production [`DefaultTextMeasure`](sigil_app::DefaultTextMeasure)
/// instead of the constant 40x16 stub, so an audit run reveals which
/// goldens depend on true glyph advances (their layout shifts). Off by
/// default so ordinary golden runs stay deterministic and stub-fast.
#[must_use]
pub fn real_measure_enabled() -> bool {
    matches!(
        std::env::var("SIGIL_REAL_MEASURE").ok().as_deref(),
        Some("1" | "true" | "yes" | "on"),
    )
}

/// The measure `render_widget_to_png` installs: the production measure
/// under `SIGIL_REAL_MEASURE=1`, otherwise the deterministic stub.
fn default_text_measure() -> Box<dyn sigil_layout::TextMeasure> {
    if real_measure_enabled() {
        Box::new(sigil_app::DefaultTextMeasure::new())
    } else {
        Box::new(StubMeasure)
    }
}

/// Resolve a golden PNG path relative to this crate.
#[must_use]
pub fn golden_path(stem: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("goldens");
    p.push(format!("{stem}.png"));
    p
}

/// Full render → image::RgbaImage. `build` constructs the widget; `mode` drives
/// the state into one of the five interaction modes through `EventRouter`.
///
/// `hit_point` is the logical-pixel point used to drive Hover / Pressed events.
/// Choose it so the pointer lands inside the widget's hit-test surface.
///
/// Skips with `None` if no wgpu adapter is available on the host.
#[must_use]
pub fn render_widget_to_png(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
) -> Option<image::RgbaImage> {
    render_widget_to_png_with(build, mode, size_px, hit_point, default_text_measure())
}

/// Same as [`render_widget_to_png`] but lets the caller inject a
/// concrete [`sigil_layout::TextMeasure`] instead of the deterministic
/// `StubMeasure`. Use this when the golden's correctness depends on
/// real glyph-advance widths (e.g. the live `widget-playground`
/// capture, which has to match what the user sees on screen).
#[must_use]
pub fn render_widget_to_png_with(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    text_measure: Box<dyn sigil_layout::TextMeasure>,
) -> Option<image::RgbaImage> {
    render_core(build, mode, size_px, hit_point, text_measure, |_, root| {
        root
    })
}

/// DPI-scale capture variant of [`render_widget_to_png_with`]: renders
/// `build` into a LOGICAL viewport of `logical_size_px`, but captures into
/// a PHYSICAL texture of `logical_size_px * scale` (rounded to the nearest
/// pixel). Use this to exercise the DPI-scale axis from the internal
/// text-quality / gap-closure plan.
///
/// As of Wave 1, the offscreen path is DPI-aware: the physical/logical ratio
/// is threaded through as [`OffscreenTarget::scale`], so captures at
/// `scale != 1.0` render the text AA ramp one *physical* pixel wide (sharp),
/// not the Wave-0 stretched/soft baseline.
#[must_use]
pub fn render_widget_to_png_scaled(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    logical_size_px: [u32; 2],
    scale: f32,
    hit_point: (f32, f32),
    text_measure: Box<dyn sigil_layout::TextMeasure>,
) -> Option<image::RgbaImage> {
    let logical = [logical_size_px[0] as f32, logical_size_px[1] as f32];
    let physical = [
        (logical[0] * scale).round() as u32,
        (logical[1] * scale).round() as u32,
    ];
    render_core_scaled(
        build,
        mode,
        logical,
        physical,
        hit_point,
        text_measure,
        |_, root| root,
    )
}

/// Same as [`render_widget_to_png`], but for composite widgets whose
/// render root is not itself the focusable primitive (e.g. a modal
/// wrapped in a coverage-panel backing container for the golden's
/// corner-clear-color trick, where the panel — not the modal's dialog
/// frame — is what `build` returns).
///
/// `InteractionMode::Focused` / `FocusedAfterFrames` route focus to the
/// first focusable descendant of `root` in document order instead of
/// `root` itself. Falls back to `root` when the subtree has no
/// focusable node, matching [`render_widget_to_png`]'s behavior.
#[must_use]
pub fn render_widget_to_png_focus_descendant(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
) -> Option<image::RgbaImage> {
    render_core(
        build,
        mode,
        size_px,
        hit_point,
        Box::new(StubMeasure),
        |cx, root| find_first_focusable(cx, root).unwrap_or(root),
    )
}

/// Shared render pipeline: build → layout → drive state (focused via
/// `focus_target`) → re-layout → emit scene → rasterize. Delegates to
/// [`render_core_scaled`] with logical viewport == physical texture size
/// (scale 1.0), which is exactly what every pre-existing caller expects —
/// this keeps their output byte-identical.
fn render_core(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    size_px: [u32; 2],
    hit_point: (f32, f32),
    text_measure: Box<dyn sigil_layout::TextMeasure>,
    focus_target: impl Fn(&Cx, NodeId) -> NodeId,
) -> Option<image::RgbaImage> {
    let logical = [size_px[0] as f32, size_px[1] as f32];
    render_core_scaled(
        build,
        mode,
        logical,
        size_px,
        hit_point,
        text_measure,
        focus_target,
    )
}

/// Generalization of [`render_core`] over a distinct LOGICAL viewport and
/// PHYSICAL capture-texture size, enabling DPI-scale capture: build →
/// layout (at `logical_size`) → drive state → re-layout → emit scene →
/// rasterize into a `physical_size_px` texture.
fn render_core_scaled(
    build: impl FnOnce(&mut Cx) -> NodeId,
    mode: InteractionMode,
    logical_size: [f32; 2],
    physical_size_px: [u32; 2],
    hit_point: (f32, f32),
    text_measure: Box<dyn sigil_layout::TextMeasure>,
    focus_target: impl Fn(&Cx, NodeId) -> NodeId,
) -> Option<image::RgbaImage> {
    let mut image: Option<image::RgbaImage> = None;
    sigil::reactive::Runtime::scope("sigil-visual-golden", || {
        let mut cx = Cx::new_root();
        let root = build(&mut cx);
        let target = focus_target(&cx, root);

        let [viewport_w, viewport_h] = logical_size;
        let mut layout = LayoutEngine::with_text_measure(text_measure);
        layout
            .compute(&mut cx, root, Size::fixed(viewport_w, viewport_h))
            .expect("layout");

        let mut router = EventRouter::new();
        drive_state(&mut router, &mut cx, &layout, target, mode, hit_point);

        // Re-run layout: state changes may have updated reactive size signals.
        layout
            .compute(&mut cx, root, Size::fixed(viewport_w, viewport_h))
            .expect("layout-after-state");

        let scene = emit_scene(&cx, root);
        let (device, queue) = request_device();
        image = Some(render_scene_to_image(
            &device,
            &queue,
            &scene,
            (viewport_w, viewport_h),
            physical_size_px,
        ));
    });
    image
}

/// Depth-first search for the first focusable descendant of `id`
/// (inclusive of `id` itself), in document order.
fn find_first_focusable(cx: &Cx, id: NodeId) -> Option<NodeId> {
    let node = cx.get(id)?;
    if node.a11y.focusable {
        return Some(id);
    }
    node.children()
        .iter()
        .find_map(|child| find_first_focusable(cx, *child))
}

/// Either assert the captured image matches `golden_path`, or — when
/// `SIGIL_REGEN_GOLDENS=1` — overwrite the golden file with `actual`.
///
/// `max_delta_pct` is the maximum percentage of differing pixels permitted
/// (range 0.0..=100.0). A pixel is "different" when any RGBA channel differs
/// by more than `CHANNEL_TOLERANCE`.
pub fn assert_png_matches_golden(actual: &image::RgbaImage, golden: &Path, max_delta_pct: f32) {
    const CHANNEL_TOLERANCE: u8 = 2;

    if regen_enabled() {
        guard_regen_does_not_erase_content(actual, golden);
        std::fs::create_dir_all(golden.parent().expect("golden parent")).expect("mkdir goldens");
        actual.save(golden).expect("write golden png");
        println!("REGEN: wrote {}", golden.display());
        return;
    }

    assert!(
        golden.exists(),
        "golden file missing: {}\n\
         Run with SIGIL_REGEN_GOLDENS=1 to create initial bakes.",
        golden.display(),
    );

    let expected = image::open(golden)
        .unwrap_or_else(|err| panic!("decode golden {}: {err}", golden.display()))
        .to_rgba8();

    assert_eq!(
        actual.dimensions(),
        expected.dimensions(),
        "golden dimension mismatch for {}",
        golden.display(),
    );

    let total = u64::from(actual.width()) * u64::from(actual.height());
    let mut differing: u64 = 0;
    for (a, b) in actual.pixels().zip(expected.pixels()) {
        let diff =
            a.0.iter()
                .zip(b.0.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE);
        if diff {
            differing += 1;
        }
    }
    let pct = (differing as f64 / total as f64) * 100.0;
    assert!(
        pct as f32 <= max_delta_pct,
        "golden {} differs in {:.3}% of pixels (>{} channel delta), \
         budget {:.3}%. Re-bake with SIGIL_REGEN_GOLDENS=1 if intentional.",
        golden.display(),
        pct,
        CHANNEL_TOLERANCE,
        max_delta_pct,
    );
}

/// Assert that two committed golden PNGs (by stem, resolved through
/// [`golden_path`]) are byte-different.
///
/// Guards the ghost-pass failure mode where a state-driven golden
/// (hover / focus / disabled / ...) accidentally renders identical to
/// its baseline counterpart: the state transition the pair exists to
/// capture authors no visible difference, so a plain `check()` call
/// Count of pixels that are not the image's single most common colour —
/// a proxy for "how much is actually drawn here".
fn ink(image: &image::RgbaImage) -> usize {
    use std::collections::HashMap;
    let mut census: HashMap<[u8; 4], usize> = HashMap::new();
    for px in image.pixels() {
        *census.entry(px.0).or_default() += 1;
    }
    let background = census.values().copied().max().unwrap_or(0);
    (image.width() as usize * image.height() as usize).saturating_sub(background)
}

/// Refuse a re-bake that erases most of what the widget used to draw.
///
/// A re-bake overwrites the reference with whatever just rendered, so a
/// regression that DELETES content certifies itself: the suite goes green
/// on an image of the bug. This happened twice on 2026-07-20 — a dropdown
/// whose label vanished, and a page collapsed to its left margin — and both
/// times the tests passed after regenerating.
///
/// Losing most of the drawn content is nearly always a bug rather than an
/// intended redesign, so it must be acknowledged explicitly via
/// `SIGIL_REGEN_ALLOW_CONTENT_LOSS=1`.
fn guard_regen_does_not_erase_content(actual: &image::RgbaImage, golden: &Path) {
    const RETAINED_FRACTION: f32 = 0.5;

    if std::env::var("SIGIL_REGEN_ALLOW_CONTENT_LOSS").as_deref() == Ok("1") {
        return;
    }
    let Ok(previous) = image::open(golden) else {
        return; // first bake: nothing to compare against
    };
    let before = ink(&previous.to_rgba8());
    let after = ink(actual);
    #[allow(clippy::cast_precision_loss)]
    let retained = if before == 0 {
        1.0
    } else {
        after as f32 / before as f32
    };
    assert!(
        retained >= RETAINED_FRACTION,
        "re-bake of {} would erase {:.0}% of the drawn content \
         ({before} -> {after} non-background pixels).\n\
         A re-bake overwrites the reference, so a regression that deletes \
         content would certify itself. Look at the render before accepting \
         it; if the reduction really is intended, set \
         SIGIL_REGEN_ALLOW_CONTENT_LOSS=1.",
        golden.display(),
        (1.0 - retained) * 100.0,
    );
}

/// passes green while certifying nothing. See
/// BUG-2026-07-02-modal-tabs-focus-states-render-nothing.
pub fn assert_goldens_differ(name_a: &str, name_b: &str) {
    let path_a = golden_path(name_a);
    let path_b = golden_path(name_b);
    let bytes_a = std::fs::read(&path_a)
        .unwrap_or_else(|err| panic!("read golden {}: {err}", path_a.display()));
    let bytes_b = std::fs::read(&path_b)
        .unwrap_or_else(|err| panic!("read golden {}: {err}", path_b.display()));
    assert_ne!(
        bytes_a, bytes_b,
        "goldens '{name_a}' and '{name_b}' are byte-identical — the state \
         change they exist to capture renders no visible difference. Fix the \
         widget, then re-bake with SIGIL_REGEN_GOLDENS=1.",
    );
}

// ---- state driving -----------------------------------------------------------

fn drive_state(
    router: &mut EventRouter,
    cx: &mut Cx,
    layout: &LayoutEngine,
    focus_target: NodeId,
    mode: InteractionMode,
    hit: (f32, f32),
) {
    match mode {
        InteractionMode::Default | InteractionMode::Disabled => {}
        InteractionMode::Hover => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
        }
        InteractionMode::Pressed => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
        }
        InteractionMode::Clicked => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
            router.route(cx, layout, pointer_up(hit.0, hit.1));
        }
        InteractionMode::Focused => {
            router.set_focus(cx, Some(focus_target));
        }
        InteractionMode::HoverAfterFrames { frames, dt } => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            tick_animation_runtime(frames, dt);
        }
        InteractionMode::PressedAfterFrames { frames, dt } => {
            router.route(cx, layout, pointer_move(hit.0, hit.1));
            router.route(cx, layout, pointer_down(hit.0, hit.1));
            tick_animation_runtime(frames, dt);
        }
        InteractionMode::FocusedAfterFrames { frames, dt } => {
            router.set_focus(cx, Some(focus_target));
            tick_animation_runtime(frames, dt);
        }
    }
}

/// Advance `sigil_animation_runtime::tick(dt)` `frames` times. When
/// `frames == u32::MAX` ticks until every active animator settles, with
/// [`SETTLE_SAFETY_CAP`] guarding against runaway loops.
fn tick_animation_runtime(frames: u32, dt: f32) {
    if frames == u32::MAX {
        for _ in 0..SETTLE_SAFETY_CAP {
            if sigil_animation_runtime::active_count() == 0 {
                return;
            }
            sigil_animation_runtime::tick(dt);
        }
        let remaining = sigil_animation_runtime::active_count();
        assert!(
            remaining == 0,
            "golden harness: animation runtime did not settle within \
             SETTLE_SAFETY_CAP ({SETTLE_SAFETY_CAP}) frames at dt={dt}. \
             Active animators remaining: {remaining}. A widget under test is \
             likely retargeting every frame.",
        );
        return;
    }
    for _ in 0..frames {
        sigil_animation_runtime::tick(dt);
    }
}

fn pointer_move(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        phase: PointerPhase::Move,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Down,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
    }))
}

fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Up,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

// ---- wgpu offscreen plumbing -------------------------------------------------

/// Acquire the offscreen wgpu device, panicking loudly when no adapter is
/// available. Loud-skip doctrine: a golden suite that silently returns
/// `None` and lets every `#[test]` pass green on a machine with no GPU is a
/// ghost-pass — the tests certify nothing while reporting success. Run the
/// golden suite only where a real adapter exists (CI without a GPU must mark
/// these `--ignored`, never let them pass vacuously).
fn request_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .block_on()
        .expect("no wgpu adapter — visual goldens need a real GPU (run --ignored on headless CI)");
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("visual-golden device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .expect("wgpu adapter has no usable device for the visual-golden suite")
}

fn render_scene_to_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
    viewport: (f32, f32),
    size_px: [u32; 2],
) -> image::RgbaImage {
    let texture = create_capture_texture(device, size_px[0], size_px[1]);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let clear = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    // Physical-per-logical ratio of this capture: the width of the physical
    // texture over the width of the logical viewport. Plain (non-scaled)
    // captures pass physical == logical, so this is exactly 1.0 and their
    // output stays byte-identical; DPI captures pass a larger physical
    // texture, so the offscreen path scales the text AA ramp for sharpness.
    let scale = size_px[0] as f32 / viewport.0;

    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport,
                scale,
            },
            scene,
            clear,
        )
        .expect("render_command_scene_offscreen");

    let raw = read_pixels(device, queue, &texture, size_px[0], size_px[1]);
    image::RgbaImage::from_raw(size_px[0], size_px[1], raw).expect("rgba image from raw")
}

fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("visual-golden capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let unpadded_bpr = width * BYTES_PER_PIXEL;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bpr = {
        let r = unpadded_bpr % align;
        if r == 0 {
            unpadded_bpr
        } else {
            unpadded_bpr + align - r
        }
    };
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("visual-golden readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("visual-golden copy"),
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bpr),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(core::iter::once(enc.finish()));

    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll");
    rx.recv().expect("map callback").expect("map result");

    let mapped = slice.get_mapped_range().expect("mapped range");
    let un = unpadded_bpr as usize;
    let pad = padded_bpr as usize;
    let h = height as usize;
    let mut out = vec![0u8; un * h];
    for row in 0..h {
        out[row * un..row * un + un].copy_from_slice(&mapped[row * pad..row * pad + un]);
    }
    drop(mapped);
    buffer.unmap();
    out
}
