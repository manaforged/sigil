#![allow(clippy::redundant_pub_crate)]

mod bands;
pub(crate) mod builtin;
mod icon;
mod parser;
mod quadratic;
mod raster;

pub use icon::Icon;
#[cfg(any(test, feature = "kittest"))]
pub use quadratic::{QuadCurve, cubic_to_quadratics};

#[cfg(test)]
#[path = "icons/lib_tests.rs"]
mod tests;

pub(crate) const MAX_PATH_SEGMENTS: u32 = 4096;

kittest_pub! {
/// Rasterizes an SVG icon into an alpha mask of the given size.
#[doc(hidden)]
pub fn bake(svg_source: &str, width: u32, height: u32) -> Result<BakedIcon, crate::Error> {
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

kittest_pub! {
/// Holds a baked icon as one alpha byte per pixel.
#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct BakedIcon {
    /// The width.
    pub width: u32,
    /// The height.
    pub height: u32,
    /// The alpha.
    pub alpha: Vec<u8>,
}
}
