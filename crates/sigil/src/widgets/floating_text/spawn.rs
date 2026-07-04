//! Spawn request + live-particle types for the floating-text pool.

use style::Rgba8;

/// One spawn request — the inputs the caller provides per particle.
#[derive(Clone, Debug, PartialEq)]
pub struct FctSpawn {
    /// Spawn position in widget-local logical pixels (`[x, y]`).
    pub at: [f32; 2],
    /// Text to render. Empty strings spawn an inert particle (no-op).
    pub text: String,
    /// Fill colour for the glyph run. Ignored when the widget was
    /// configured with [`super::FloatingTextBuilder::style`] — the
    /// resolved [`super::FloaterStyle`] color wins.
    pub color: Rgba8,
}

impl FctSpawn {
    /// Convenience constructor.
    #[must_use]
    pub fn new(at: [f32; 2], text: impl Into<String>, color: Rgba8) -> Self {
        Self {
            at,
            text: text.into(),
            color,
        }
    }
}

impl Default for FctSpawn {
    fn default() -> Self {
        Self {
            at: [0.0, 0.0],
            text: String::new(),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }
    }
}

/// A single live particle. `dead` slots are reused by the next spawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    /// Origin point at spawn time.
    pub origin: [f32; 2],
    /// Drift velocity captured at spawn time (px/s).
    pub drift: [f32; 2],
    /// Current position (updated each tick).
    pub position: [f32; 2],
    /// Spawn time in seconds (from the caller's clock).
    pub spawn_time: f32,
    /// Remaining lifetime in seconds. Reaches `0.0` → slot reclaimed.
    pub remaining: f32,
    /// Alpha in `[0.0, 1.0]`. Eases to 0 over the lifetime.
    pub alpha: f32,
    /// Fill colour. Alpha channel is multiplied with `alpha` at read.
    pub color: Rgba8,
    /// Glyph text — short enough that `String` allocation does not
    /// dominate hot-path cost for typical 1-6 character damage numbers.
    pub text: String,
    /// Slot occupied flag.
    pub alive: bool,
}

impl Particle {
    /// An inert empty slot.
    pub(super) const fn empty() -> Self {
        Self {
            origin: [0.0, 0.0],
            drift: [0.0, 0.0],
            position: [0.0, 0.0],
            spawn_time: 0.0,
            remaining: 0.0,
            alpha: 0.0,
            color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            text: String::new(),
            alive: false,
        }
    }
}
