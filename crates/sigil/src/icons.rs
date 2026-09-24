#![allow(clippy::redundant_pub_crate)]

mod bands;
pub mod builtin;
mod error;
mod parser;
mod quadratic;
mod raster;

pub use error::BakeError;
pub use quadratic::{QuadCurve, cubic_to_quadratics};

#[cfg(test)]
#[path = "icons/lib_tests.rs"]
mod tests;

pub const MAX_PATH_SEGMENTS: u32 = 4096;

#[derive(Debug, Default)]
pub struct IconBaker {
    _private: (),
}

impl IconBaker {
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

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

#[derive(Clone, Debug)]
pub struct BakedIcon {
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
}
