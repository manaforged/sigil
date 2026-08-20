//! Runtime-adjustable text-rendering tuning for the Slug pipeline.
//!
//! The crispness levers (dilation, AA-ramp sharpening, stem-darkening gamma,
//! pixel snapping) were fixed compile-time constants. Exposing them as a small
//! process-global lets a debug panel A/B them live against the running window —
//! flip `enabled` to compare tuned ("cleartext") rendering against raw Slug, or
//! drag a single lever to find its sweet spot without a rebuild.
//!
//! Stored as lock-free atomics (floats via bit-pattern) because [`text_tuning`]
//! is read every frame from the paint thread while the panel writes occasionally
//! from the event thread.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// A snapshot of every text-rendering knob. Read once per paragraph at paint
/// time; written by UI controls. Defaults are the shipped tuned values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTuning {
    /// Master A/B switch. `false` renders raw Slug (no dilation, unit-width AA
    /// ramp, no stem darkening, no snapping) — the comparison baseline.
    pub enabled: bool,
    /// Snap the baseline and each glyph's left ink edge to the physical grid.
    pub snap: bool,
    /// Quad dilation in physical pixels (captures partially-covered border pixels).
    pub dilation: f32,
    /// AA-ramp sharpening factor (>1 narrows the ramp → crisper, harder edges).
    pub sharpen: f32,
    /// Stem-darkening gamma for LIGHT-on-dark text (bright glyphs over a dark
    /// background), applied as `pow(coverage, 1/gamma)` (1.0 = off).
    ///
    /// Light-on-dark is the polarity linear-space sRGB coverage blending thins
    /// the most, so this is the stronger of the two corrections. Supersedes the
    /// former single `gamma` field: the old `gamma = 1.5` was tuned against the
    /// light-on-dark stem-check fixture, so it maps here.
    pub gamma_light_on_dark: f32,
    /// Stem-darkening gamma for DARK-on-light text (dark glyphs over a light
    /// background), applied as `pow(coverage, 1/gamma)` (1.0 = off).
    ///
    /// Higher than [`Self::gamma_light_on_dark`]: linear-space sRGB blending
    /// renders dark-on-light stems the *thinner* of the two polarities, so it
    /// needs the stronger darkening to land at matched perceived stem weight.
    /// `2.5` is the empirical value from the `glyph_pixel_check` polarity pair
    /// (stroke-weight parity within ~7% at scale 1.0, ~4% at scale 2.0).
    pub gamma_dark_on_light: f32,
}

impl Default for TextTuning {
    fn default() -> Self {
        Self {
            enabled: true,
            snap: true,
            dilation: 0.5,
            sharpen: 2.0,
            gamma_light_on_dark: 1.5,
            gamma_dark_on_light: 2.5,
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static SNAP: AtomicBool = AtomicBool::new(true);
static DILATION: AtomicU32 = AtomicU32::new(0x3F00_0000); // 0.5
static SHARPEN: AtomicU32 = AtomicU32::new(0x4000_0000); // 2.0
static GAMMA_LOD: AtomicU32 = AtomicU32::new(0x3FC0_0000); // 1.5
static GAMMA_DOL: AtomicU32 = AtomicU32::new(0x4020_0000); // 2.5

/// Read the current text-tuning snapshot. Called every frame from paint.
#[must_use]
pub fn text_tuning() -> TextTuning {
    TextTuning {
        enabled: ENABLED.load(Ordering::Relaxed),
        snap: SNAP.load(Ordering::Relaxed),
        dilation: f32::from_bits(DILATION.load(Ordering::Relaxed)),
        sharpen: f32::from_bits(SHARPEN.load(Ordering::Relaxed)),
        gamma_light_on_dark: f32::from_bits(GAMMA_LOD.load(Ordering::Relaxed)),
        gamma_dark_on_light: f32::from_bits(GAMMA_DOL.load(Ordering::Relaxed)),
    }
}

/// Replace the text-tuning state. The next painted frame reflects it.
pub fn set_text_tuning(tuning: TextTuning) {
    ENABLED.store(tuning.enabled, Ordering::Relaxed);
    SNAP.store(tuning.snap, Ordering::Relaxed);
    DILATION.store(tuning.dilation.to_bits(), Ordering::Relaxed);
    SHARPEN.store(tuning.sharpen.to_bits(), Ordering::Relaxed);
    GAMMA_LOD.store(tuning.gamma_light_on_dark.to_bits(), Ordering::Relaxed);
    GAMMA_DOL.store(tuning.gamma_dark_on_light.to_bits(), Ordering::Relaxed);
}
