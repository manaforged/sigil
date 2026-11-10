#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "noise lattice coordinates are floored floats that fit in i32"
)]

use crate::style::Rgba8;

#[cfg(feature = "scene")]
mod noise;
#[cfg(feature = "scene")]
mod paint;

#[cfg(all(
    feature = "scene",
    any(test, feature = "kittest", feature = "desktop", feature = "terminal")
))]
/// Packs an area's centre in whole pixels into the bits of one float for the shader.
pub(crate) fn centre_bits(width: f32, height: f32) -> f32 {
    let half = |extent: f32| (extent / 2.0).round().clamp(0.0, 65_535.0) as u32;
    f32::from_bits(half(width) | (half(height) << 16))
}

/// The most pigments one wash carries.
pub(crate) const MAX_PIGMENTS: usize = 4;
const SEED_MASK: u32 = 0x00ff_ffff;

/// A watercolour wash: pigment pooled over paper, or light over a night sky when the paper is dark.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wash {
    pub(crate) paper: Rgba8,
    pub(crate) pigments: [Rgba8; MAX_PIGMENTS],
    pub(crate) count: u8,
    pub(crate) seed: u32,
    pub(crate) scale: f32,
    pub(crate) density: f32,
    pub(crate) pools: f32,
    pub(crate) edges: f32,
    pub(crate) granulation: f32,
    pub(crate) grain: f32,
    pub(crate) specks: f32,
    pub(crate) band: bool,
}

impl Wash {
    /// A wash on `paper` with no pigment yet; the paper's alpha sets the opacity of the whole wash.
    #[must_use]
    pub const fn new(paper: Rgba8) -> Self {
        Self {
            paper,
            pigments: [paper; MAX_PIGMENTS],
            count: 0,
            seed: 7,
            scale: 260.0,
            density: 1.0,
            pools: 6.0,
            edges: 0.6,
            granulation: 0.5,
            grain: 0.5,
            specks: 0.0,
            band: false,
        }
    }

    /// Adds a pigment, as it looks at full strength; past four it replaces the last.
    #[must_use]
    pub const fn pigment(mut self, color: Rgba8) -> Self {
        let at = if (self.count as usize) < MAX_PIGMENTS {
            self.count += 1;
            self.count as usize - 1
        } else {
            MAX_PIGMENTS - 1
        };
        if let Some((_, [slot, ..])) = self.pigments.split_at_mut_checked(at) {
            *slot = color;
        }
        self
    }

    /// Sets the seed that picks the shapes.
    #[must_use]
    pub const fn seed(mut self, seed: u32) -> Self {
        self.seed = seed & SEED_MASK;
        self
    }

    /// Sets the size of the largest shapes in pixels.
    #[must_use]
    pub const fn scale(mut self, pixels: f32) -> Self {
        self.scale = pixels;
        self
    }

    /// Sets how much pigment the paper takes, from 0 to 2.
    #[must_use]
    pub const fn density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Sets how many flat pools the pigment settles into; 0 keeps it smooth.
    #[must_use]
    pub const fn pools(mut self, pools: u8) -> Self {
        self.pools = pools as f32;
        self
    }

    /// Sets how dark pigment dries along the edge of each pool, from 0 to 1.
    #[must_use]
    pub const fn edges(mut self, edges: f32) -> Self {
        self.edges = edges;
        self
    }

    /// Sets how much pigment clumps into fine grains, from 0 to 1.
    #[must_use]
    pub const fn granulation(mut self, granulation: f32) -> Self {
        self.granulation = granulation;
        self
    }

    /// Sets how strongly the paper fibres show, from 0 to 1.
    #[must_use]
    pub const fn grain(mut self, grain: f32) -> Self {
        self.grain = grain;
        self
    }

    /// Sets the density of ink specks on light paper or stars on dark paper, from 0 to 1.
    #[must_use]
    pub const fn specks(mut self, specks: f32) -> Self {
        self.specks = specks;
        self
    }

    /// Gathers the pigment into one drifting band across the area, like a galaxy.
    #[must_use]
    pub const fn band(mut self, band: bool) -> Self {
        self.band = band;
        self
    }

    /// Returns this wash with its values in range.
    #[must_use]
    pub const fn normalized(mut self) -> Self {
        self.scale = if self.scale.is_finite() {
            self.scale.max(8.0)
        } else {
            260.0
        };
        self.density = finite_clamp(self.density, 0.0, 2.0);
        self.pools = finite_clamp(self.pools, 0.0, 16.0);
        self.edges = finite_clamp(self.edges, 0.0, 1.0);
        self.granulation = finite_clamp(self.granulation, 0.0, 1.0);
        self.grain = finite_clamp(self.grain, 0.0, 1.0);
        self.specks = finite_clamp(self.specks, 0.0, 1.0);
        self
    }
}

const fn finite_clamp(value: f32, lo: f32, hi: f32) -> f32 {
    if value.is_finite() {
        value.clamp(lo, hi)
    } else {
        lo
    }
}
