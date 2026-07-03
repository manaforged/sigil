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
    /// Stem-darkening gamma applied as `pow(coverage, 1/gamma)` (1.0 = off).
    pub gamma: f32,
}

impl Default for TextTuning {
    fn default() -> Self {
        Self {
            enabled: true,
            snap: true,
            dilation: 0.5,
            sharpen: 2.0,
            gamma: 1.5,
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static SNAP: AtomicBool = AtomicBool::new(true);
static DILATION: AtomicU32 = AtomicU32::new(0x3F00_0000); // 0.5
static SHARPEN: AtomicU32 = AtomicU32::new(0x4000_0000); // 2.0
static GAMMA: AtomicU32 = AtomicU32::new(0x3FC0_0000); // 1.5

/// Read the current text-tuning snapshot. Called every frame from paint.
#[must_use]
pub fn text_tuning() -> TextTuning {
    TextTuning {
        enabled: ENABLED.load(Ordering::Relaxed),
        snap: SNAP.load(Ordering::Relaxed),
        dilation: f32::from_bits(DILATION.load(Ordering::Relaxed)),
        sharpen: f32::from_bits(SHARPEN.load(Ordering::Relaxed)),
        gamma: f32::from_bits(GAMMA.load(Ordering::Relaxed)),
    }
}

/// Replace the text-tuning state. The next painted frame reflects it.
pub fn set_text_tuning(tuning: TextTuning) {
    ENABLED.store(tuning.enabled, Ordering::Relaxed);
    SNAP.store(tuning.snap, Ordering::Relaxed);
    DILATION.store(tuning.dilation.to_bits(), Ordering::Relaxed);
    SHARPEN.store(tuning.sharpen.to_bits(), Ordering::Relaxed);
    GAMMA.store(tuning.gamma.to_bits(), Ordering::Relaxed);
}
