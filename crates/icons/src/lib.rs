//! `sigil-icons` — SVG-to-alpha-mask icon baker for Sigil.
// All sub-modules are private; pub(crate) inside a private module is
// flagged as redundant by clippy. Allow it crate-wide so internal modules
// can use pub(crate) for clarity without triggering the lint.
#![allow(clippy::redundant_pub_crate)]
//!
//! Converts SVG path data into a flat `Vec<u8>` alpha mask via:
//! 1. `usvg` parse + transform-flatten (`parser`)
//! 2. cubic→quadratic approximation via `lyon_geom` (`quadratic`)
//! 3. Slug-style 8-band vertical partition (`bands`)
//! 4. 4×4 SSAA CPU rasterizer → box-filter (`raster`)
//!
//! # Example
//!
//! ```rust
//! use sigil_icons::IconBaker;
//!
//! let baker = IconBaker::new();
//! let icon = baker.bake(r#"<svg viewBox="0 0 24 24"><path d="M12 2L2 22h20Z"/></svg>"#, 32, 32).unwrap();
//! assert_eq!(icon.alpha.len(), 32 * 32);
//! ```

mod bands;
pub mod builtin;
mod error;
mod parser;
mod quadratic;
mod raster;

pub use error::BakeError;
pub use quadratic::{QuadCurve, cubic_to_quadratics};

#[cfg(test)]
use proptest as _;

/// Maximum number of quadratic segments per bake call.
/// Exceeding this limit returns [`BakeError::TooManySegments`].
pub const MAX_PATH_SEGMENTS: u32 = 4096;

/// Bakes SVG sources into single-channel alpha masks.
///
/// `IconBaker` is cheap to construct and stateless; create one per
/// application and reuse across multiple [`bake`][`IconBaker::bake`] calls.
#[derive(Debug, Default)]
pub struct IconBaker {
    _private: (),
}

impl IconBaker {
    /// Create a new [`IconBaker`].
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Bake an SVG source string into a single-icon alpha mask.
    ///
    /// Returns a [`BakedIcon`] whose `alpha` slice has exactly
    /// `width * height` bytes, each in `0..=255`.
    ///
    /// # Errors
    ///
    /// Returns [`BakeError`] on malformed SVG, missing viewBox,
    /// path-coordinate overflow, or excessive segment count.
    pub fn bake(&self, svg_source: &str, width: u32, height: u32) -> Result<BakedIcon, BakeError> {
        let (paths, view_box) = parser::parse(svg_source)?;
        let quads = quadratic::to_quads(&paths)?;
        let band_buf = bands::build(&quads, view_box);
        let alpha = raster::rasterize(&band_buf, view_box, width, height);
        Ok(BakedIcon {
            width,
            height,
            alpha,
        })
    }
}

/// A baked icon: a single-channel alpha mask ready for atlas upload.
#[derive(Clone, Debug)]
pub struct BakedIcon {
    /// Width of the mask in pixels.
    pub width: u32,
    /// Height of the mask in pixels.
    pub height: u32,
    /// Alpha values, row-major, `len == width * height`.
    pub alpha: Vec<u8>,
}
