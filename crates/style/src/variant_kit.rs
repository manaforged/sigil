//! Pre-baked visual variants for tone-driven widgets.
//!
//! A variant defines the WHAT — solid fill vs. soft tint vs. outline.
//! The tone defines the WHICH — neutral vs. primary vs. error. The
//! pair (tone, variant) plus the runtime interaction state resolves
//! through the engine to concrete colors.
//!
//! Variants are tone-agnostic: every atom in here uses
//! [`StyleAtom::BgToneStep`](crate::StyleAtom) /
//! [`StyleAtom::FgToneStep`](crate::StyleAtom), which resolve through
//! the node's [`ToneContext`](crate::ToneContext) extension at apply
//! time. The same `SOLID_VARIANT` paints accent-9 for `Tone::Primary`
//! and error-9 for `Tone::Error`.
//!
//! Adding a tone is a two-line change (variant on the [`Tone`](crate::Tone)
//! enum + scale field on [`Theme`](crate::Theme)) — variant tables
//! don't change.

use crate::{StateStyle, StyleAtom, StyleVariant};

// === Solid variant ============================================================
//
// Saturated tone fill. Step 9 base, step 10 hover, step 11 pressed.
// Step 8 + 50% opacity for disabled. Text uses each scale's
// `on_solid` contrast color — guarantees WCAG-AA against step 9
// regardless of tone (the previous `FgToneStep(12)` failed contrast
// for warm tones like amber in dark mode).

static SOLID_BASE: &[StyleAtom] = &[StyleAtom::BgToneStep(9), StyleAtom::FgOnSolid];
static SOLID_REST: &[StyleAtom] = &[];
static SOLID_FOCUS: &[StyleAtom] = &[StyleAtom::BgToneStep(10)];
static SOLID_HOVER: &[StyleAtom] = &[StyleAtom::BgToneStep(10)];
static SOLID_PRESSED: &[StyleAtom] = &[StyleAtom::BgToneStep(11)];
static SOLID_DISABLED: &[StyleAtom] = &[StyleAtom::BgToneStep(8), StyleAtom::Opacity(5_000)];

const SOLID_STATES: StateStyle = StateStyle::new(
    SOLID_REST,
    SOLID_FOCUS,
    SOLID_HOVER,
    SOLID_PRESSED,
    SOLID_DISABLED,
);

/// Saturated tone fill with high-contrast text. The classic
/// "primary button" look across every tone — primary, error,
/// success, info, warn, neutral all use the same shape.
pub const SOLID_VARIANT: StyleVariant = StyleVariant::new(SOLID_BASE, SOLID_STATES);

// === Soft variant =============================================================
//
// Raised surface at step 4, hover 5, pressed 6 — the elevation sits one
// step above the step-3 "element rest" so a secondary button reads as a
// raised control on a step-2 card (Zed/ShadCN separate controls by fill,
// not outline). Step 11 text for readable on-tinted contrast.

static SOFT_BASE: &[StyleAtom] = &[StyleAtom::BgToneStep(4), StyleAtom::FgToneStep(11)];
static SOFT_REST: &[StyleAtom] = &[];
static SOFT_FOCUS: &[StyleAtom] = &[StyleAtom::BgToneStep(5)];
static SOFT_HOVER: &[StyleAtom] = &[StyleAtom::BgToneStep(5)];
static SOFT_PRESSED: &[StyleAtom] = &[StyleAtom::BgToneStep(6)];
static SOFT_DISABLED: &[StyleAtom] = &[StyleAtom::BgToneStep(2), StyleAtom::Opacity(5_000)];

const SOFT_STATES: StateStyle = StateStyle::new(
    SOFT_REST,
    SOFT_FOCUS,
    SOFT_HOVER,
    SOFT_PRESSED,
    SOFT_DISABLED,
);

/// Tinted background with readable on-tinted text. The "secondary
/// action" or "informational chip" look.
pub const SOFT_VARIANT: StyleVariant = StyleVariant::new(SOFT_BASE, SOFT_STATES);
