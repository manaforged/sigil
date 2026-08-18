//! Arbitrary vector-path rendering.
//!
//! Rasterizes a Bézier path (fill and/or stroke, anti-aliased) on the CPU
//! via `tiny-skia` into an RGBA canvas, then renders it through the built-in
//! image node — the same reuse pattern color emoji uses. This closes the
//! "no arbitrary vector paths" gap: SVG-style icons, chart shapes, sparklines,
//! and any custom 2D graphic the rect/gradient/text primitives cannot express.
//!
//! Coordinates are in the raster's pixel space (`0..width`, `0..height`).
//! Equal command lists reuse one CPU raster. The renderer keys GPU
//! uploads on [`ImageSource`] id, so a remount does not re-upload.
//!
//! ```rust,no_run
//! use sigil::prelude::*;
//! use sigil_style::Rgba8;
//!
//! // A filled triangle.
//! let accent = Rgba8 { r: 80, g: 120, b: 255, a: 255 };
//! let _tri = path(48, 48)
//!     .move_to(24.0, 4.0)
//!     .line_to(44.0, 44.0)
//!     .line_to(4.0, 44.0)
//!     .close()
//!     .fill(accent);
//! ```

use std::cell::RefCell;
use std::collections::HashMap;

use sigil_kernel::{Cx, ImageSource, IntoNode, NodeId, container, image};
use sigil_layout::prelude::LayoutStyle;
use sigil_style::Rgba8;
use tiny_skia::{
    Color as TsColor, FillRule, GradientStop, LinearGradient, Paint, PathBuilder as TsPathBuilder,
    Pixmap, Point as TsPoint, RadialGradient, Shader, SpreadMode, Stroke, Transform,
};

/// A path fill: a flat color or a gradient. Gradient stops are `(position
/// in 0..1, color)`, and gradient coordinates share the path's pixel space.
pub enum PathFill {
    /// Flat fill.
    Solid(Rgba8),
    /// Linear gradient from `start` to `end`.
    Linear {
        /// Gradient axis start point, in raster pixels.
        start: (f32, f32),
        /// Gradient axis end point, in raster pixels.
        end: (f32, f32),
        /// Colour stops along the axis.
        stops: Vec<(f32, Rgba8)>,
    },
    /// Radial gradient centred at `center` reaching `radius`.
    Radial {
        /// Centre point, in raster pixels.
        center: (f32, f32),
        /// Outer radius, in raster pixels.
        radius: f32,
        /// Colour stops from centre (0.0) to edge (1.0).
        stops: Vec<(f32, Rgba8)>,
    },
}

/// One command in a vector path, in the raster's pixel coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCmd {
    /// Begin a new subpath at `(x, y)`.
    MoveTo(f32, f32),
    /// Straight line to `(x, y)`.
    LineTo(f32, f32),
    /// Quadratic Bézier: control `(cx, cy)`, end `(x, y)`.
    QuadTo(f32, f32, f32, f32),
    /// Cubic Bézier: controls `(c1x, c1y)` and `(c2x, c2y)`, end `(x, y)`.
    CubicTo(f32, f32, f32, f32, f32, f32),
    /// Close the current subpath back to its start point.
    Close,
}

/// A vector path plus its fill/stroke, rasterized to an image node on build.
///
/// Construct with [`path`], chain the segment methods, set [`fill`](Self::fill)
/// and/or [`stroke`](Self::stroke), then drop it into the tree like any node.
pub struct PathNode {
    width: u32,
    height: u32,
    cmds: Vec<PathCmd>,
    fill: Option<PathFill>,
    stroke: Option<(Rgba8, f32)>,
    layout: Option<sigil_layout::prelude::LayoutStyle>,
    scale: u8,
}

thread_local! {
    static RASTER: RefCell<HashMap<u64, ImageSource>> = RefCell::new(HashMap::new());
}

const RASTER_CAP: usize = 256;

fn mix(hash: &mut u64, byte: u8) {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    *hash ^= u64::from(byte);
    *hash = hash.wrapping_mul(PRIME);
}

fn mix_f32(hash: &mut u64, value: f32) {
    for byte in value.to_bits().to_le_bytes() {
        mix(hash, byte);
    }
}

fn mix_rgba(hash: &mut u64, color: Rgba8) {
    mix(hash, color.r);
    mix(hash, color.g);
    mix(hash, color.b);
    mix(hash, color.a);
}

fn mix_stops(hash: &mut u64, stops: &[(f32, Rgba8)]) {
    for &(pos, color) in stops {
        mix_f32(hash, pos);
        mix_rgba(hash, color);
    }
}

fn hash_cmd(hash: &mut u64, cmd: PathCmd) {
    match cmd {
        PathCmd::MoveTo(x, y) => {
            mix(hash, 1);
            mix_f32(hash, x);
            mix_f32(hash, y);
        }
        PathCmd::LineTo(x, y) => {
            mix(hash, 2);
            mix_f32(hash, x);
            mix_f32(hash, y);
        }
        PathCmd::QuadTo(cx, cy, x, y) => {
            mix(hash, 3);
            mix_f32(hash, cx);
            mix_f32(hash, cy);
            mix_f32(hash, x);
            mix_f32(hash, y);
        }
        PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
            mix(hash, 4);
            mix_f32(hash, c1x);
            mix_f32(hash, c1y);
            mix_f32(hash, c2x);
            mix_f32(hash, c2y);
            mix_f32(hash, x);
            mix_f32(hash, y);
        }
        PathCmd::Close => mix(hash, 5),
    }
}

/// Start a vector path rasterized onto a `width × height` RGBA canvas.
#[must_use]
pub const fn path(width: u32, height: u32) -> PathNode {
    PathNode {
        width,
        height,
        cmds: Vec::new(),
        fill: None,
        stroke: None,
        layout: None,
        scale: 1,
    }
}

impl PathNode {
    /// Begin a new subpath at `(x, y)`.
    #[must_use]
    pub fn move_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::MoveTo(x, y));
        self
    }

    /// Add a straight line to `(x, y)`.
    #[must_use]
    pub fn line_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::LineTo(x, y));
        self
    }

    /// Add a quadratic Bézier through control `(cx, cy)` to `(x, y)`.
    #[must_use]
    pub fn quad_to(mut self, cx: f32, cy: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::QuadTo(cx, cy, x, y));
        self
    }

    /// Add a cubic Bézier through controls `(c1x, c1y)`, `(c2x, c2y)` to `(x, y)`.
    // A cubic Bézier is six coordinates by definition; the flat SVG-style
    // signature reads better than tupling them just to satisfy the arg cap.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn cubic_to(mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y));
        self
    }

    /// Close the current subpath.
    #[must_use]
    pub fn close(mut self) -> Self {
        self.cmds.push(PathCmd::Close);
        self
    }

    /// Fill the path with a flat `color` (non-zero winding rule).
    #[must_use]
    pub fn fill(mut self, color: Rgba8) -> Self {
        self.fill = Some(PathFill::Solid(color));
        self
    }

    /// Fill the path with a linear gradient along `start` → `end`. `stops`
    /// are `(position in 0..1, color)` in gradient order.
    #[must_use]
    pub fn fill_linear(
        mut self,
        start: (f32, f32),
        end: (f32, f32),
        stops: Vec<(f32, Rgba8)>,
    ) -> Self {
        self.fill = Some(PathFill::Linear { start, end, stops });
        self
    }

    /// Fill the path with a radial gradient centred at `center` reaching
    /// `radius`. `stops` run from the centre (0.0) to the edge (1.0).
    #[must_use]
    pub fn fill_radial(
        mut self,
        center: (f32, f32),
        radius: f32,
        stops: Vec<(f32, Rgba8)>,
    ) -> Self {
        self.fill = Some(PathFill::Radial {
            center,
            radius,
            stops,
        });
        self
    }

    /// Stroke the path outline with `color` at `width` pixels.
    #[must_use]
    pub const fn stroke(mut self, color: Rgba8, width: f32) -> Self {
        self.stroke = Some((color, width));
        self
    }

    /// Pin the node's laid-out size, overriding the raster's intrinsic
    /// pixel size. Rasterize at 2x the intended logical size and pin the
    /// logical box here for crisp output on hi-dpi targets.
    #[must_use]
    pub fn layout(mut self, style: LayoutStyle) -> Self {
        self.layout = Some(style);
        self
    }

    /// Raster at 2× and pin a non-shrinking box of the path's declared
    /// size. Stroke icons stay visible in flex rows on retina.
    #[must_use]
    pub fn fit(self) -> Self {
        // Path sizes are small pixel dimensions, far below f32's exact-integer range.
        #[allow(clippy::cast_precision_loss)]
        let logical = self.width as f32;
        self.fit_size(logical)
    }

    /// Raster at 2× and pin a non-shrinking `logical` square.
    #[must_use]
    pub fn fit_size(mut self, logical: f32) -> Self {
        self.scale = 2;
        self.layout = Some(
            LayoutStyle::block()
                .fixed_size(logical, logical)
                .shrink(0.0)
                .min_size(logical, logical),
        );
        self
    }

    fn intern_key(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        let mut hash = OFFSET;
        for byte in self.width.to_le_bytes() {
            mix(&mut hash, byte);
        }
        for byte in self.height.to_le_bytes() {
            mix(&mut hash, byte);
        }
        mix(&mut hash, self.scale);
        for cmd in &self.cmds {
            hash_cmd(&mut hash, *cmd);
        }
        match self.stroke {
            Some((color, width)) => {
                mix(&mut hash, 1);
                mix_rgba(&mut hash, color);
                mix_f32(&mut hash, width);
            }
            None => mix(&mut hash, 0),
        }
        match &self.fill {
            Some(PathFill::Solid(color)) => {
                mix(&mut hash, 1);
                mix_rgba(&mut hash, *color);
            }
            Some(PathFill::Linear { start, end, stops }) => {
                mix(&mut hash, 2);
                mix_f32(&mut hash, start.0);
                mix_f32(&mut hash, start.1);
                mix_f32(&mut hash, end.0);
                mix_f32(&mut hash, end.1);
                mix_stops(&mut hash, stops);
            }
            Some(PathFill::Radial {
                center,
                radius,
                stops,
            }) => {
                mix(&mut hash, 3);
                mix_f32(&mut hash, center.0);
                mix_f32(&mut hash, center.1);
                mix_f32(&mut hash, *radius);
                mix_stops(&mut hash, stops);
            }
            None => mix(&mut hash, 0),
        }
        hash
    }

    /// Rasterize into an [`ImageSource`], or `None` if the path is empty or
    /// the canvas size is degenerate.
    fn rasterize(&self) -> Option<ImageSource> {
        let key = self.intern_key();
        if let Some(hit) = RASTER.with(|map| map.borrow().get(&key).cloned()) {
            return Some(hit);
        }
        sigil_profile::count("path.raster");
        let path = build_ts_path(&self.cmds)?;
        let scale = f32::from(self.scale.max(1));
        // Pixel dimensions are far below f32's exact-integer range and
        // scale is positive, so no sign or precision is lost.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let pix_w = ((self.width as f32) * scale).round() as u32;
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let pix_h = ((self.height as f32) * scale).round() as u32;
        let mut pixmap = Pixmap::new(pix_w, pix_h)?;
        let transform = Transform::from_scale(scale, scale);
        if let Some(fill) = &self.fill {
            let paint = Paint {
                shader: fill_shader(fill),
                anti_alias: true,
                ..Paint::default()
            };
            pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
        }
        if let Some((color, width)) = self.stroke {
            let mut paint = Paint::default();
            paint.set_color(to_ts_color(color));
            paint.anti_alias = true;
            let stroke = Stroke {
                width: width * scale,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);
        }
        let source = ImageSource::from_rgba8(pix_w, pix_h, straight_rgba(&pixmap))?;
        RASTER.with(|map| {
            let mut map = map.borrow_mut();
            if map.len() >= RASTER_CAP {
                map.clear();
            }
            map.insert(key, source.clone());
        });
        Some(source)
    }
}

impl IntoNode for PathNode {
    fn build(self, cx: &mut Cx) -> NodeId {
        match self.rasterize() {
            Some(source) => {
                let id = image(source).build(cx);
                if let Some(style) = self.layout
                    && let Some(node) = cx.get_mut(id)
                {
                    let _ = node.attach(style);
                }
                id
            }
            // Empty / degenerate path: a zero-size node so callers can still
            // place it without a special case.
            None => container().build(cx),
        }
    }
}

/// Translate accumulated commands into a `tiny-skia` path.
fn build_ts_path(cmds: &[PathCmd]) -> Option<tiny_skia::Path> {
    let mut builder = TsPathBuilder::new();
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => builder.move_to(x, y),
            PathCmd::LineTo(x, y) => builder.line_to(x, y),
            PathCmd::QuadTo(cx, cy, x, y) => builder.quad_to(cx, cy, x, y),
            PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
                builder.cubic_to(c1x, c1y, c2x, c2y, x, y);
            }
            PathCmd::Close => builder.close(),
        }
    }
    builder.finish()
}

/// `tiny-skia` renders into premultiplied RGBA; the image node wants
/// straight alpha, so demultiply every pixel back.
fn straight_rgba(pixmap: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixmap.pixels().len().saturating_mul(4));
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        out.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    out
}

fn to_ts_color(color: Rgba8) -> TsColor {
    TsColor::from_rgba8(color.r, color.g, color.b, color.a)
}

/// Build the `tiny-skia` shader for a fill, falling back to transparent if
/// a gradient is degenerate (no stops, zero radius).
fn fill_shader(fill: &PathFill) -> Shader<'static> {
    let transparent = Shader::SolidColor(TsColor::TRANSPARENT);
    match fill {
        PathFill::Solid(color) => Shader::SolidColor(to_ts_color(*color)),
        PathFill::Linear { start, end, stops } => LinearGradient::new(
            point(*start),
            point(*end),
            ts_stops(stops),
            SpreadMode::Pad,
            Transform::identity(),
        )
        .unwrap_or(transparent),
        PathFill::Radial {
            center,
            radius,
            stops,
        } => {
            let focus = point(*center);
            RadialGradient::new(
                focus,
                focus,
                *radius,
                ts_stops(stops),
                SpreadMode::Pad,
                Transform::identity(),
            )
            .unwrap_or(transparent)
        }
    }
}

fn ts_stops(stops: &[(f32, Rgba8)]) -> Vec<GradientStop> {
    stops
        .iter()
        .map(|&(position, color)| GradientStop::new(position, to_ts_color(color)))
        .collect()
}

fn point((x, y): (f32, f32)) -> TsPoint {
    TsPoint::from_xy(x, y)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn fills_a_triangle_in_color() {
        let blue = Rgba8 {
            r: 40,
            g: 90,
            b: 230,
            a: 255,
        };
        let node = path(32, 32)
            .move_to(16.0, 2.0)
            .line_to(30.0, 30.0)
            .line_to(2.0, 30.0)
            .close()
            .fill(blue);
        let source = node.rasterize().expect("triangle rasterizes");
        assert_eq!(source.width(), 32);
        assert_eq!(source.height(), 32);
        let again = path(32, 32)
            .move_to(16.0, 2.0)
            .line_to(30.0, 30.0)
            .line_to(2.0, 30.0)
            .close()
            .fill(blue)
            .rasterize()
            .unwrap();
        assert_eq!(
            source.id(),
            again.id(),
            "same commands reuse the CPU raster"
        );

        // The centroid pixel must be opaque and blue-dominant.
        let rgba = node.rasterize().unwrap();
        assert_eq!(rgba.width(), 32);
        // Re-rasterize to inspect pixels via a fresh pixmap.
        let mut pixmap = Pixmap::new(32, 32).unwrap();
        let ts_path = build_ts_path(&node.cmds).unwrap();
        let mut paint = Paint::default();
        paint.set_color(to_ts_color(blue));
        paint.anti_alias = true;
        pixmap.fill_path(
            &ts_path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        let center = pixmap.pixel(16, 20).unwrap().demultiply();
        assert!(
            center.alpha() > 200 && center.blue() > center.red(),
            "triangle interior must be opaque blue, got {center:?}"
        );
    }

    #[test]
    fn empty_path_yields_no_image() {
        assert!(
            path(16, 16)
                .fill(Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255
                })
                .rasterize()
                .is_none()
        );
    }

    #[test]
    fn fit_rasters_two_x_and_pins_logical_box() {
        let node = path(16, 16)
            .move_to(2.0, 2.0)
            .line_to(14.0, 14.0)
            .stroke(
                Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                1.35,
            )
            .fit();
        let source = node.rasterize().expect("stroke rasterizes");
        assert_eq!(source.width(), 32, "2x pixmap");
        assert_eq!(source.height(), 32);
        assert!(node.layout.is_some(), "fit pins a logical box");
    }

    #[test]
    fn intern_key_distinguishes_gradient_fills() {
        let red = Rgba8 {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        let blue = Rgba8 {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        };
        let a = path(16, 16)
            .move_to(0.0, 0.0)
            .line_to(16.0, 16.0)
            .fill_linear((0.0, 0.0), (16.0, 0.0), vec![(0.0, red), (1.0, blue)]);
        let b = path(16, 16)
            .move_to(0.0, 0.0)
            .line_to(16.0, 16.0)
            .fill_linear((0.0, 0.0), (16.0, 0.0), vec![(0.0, blue), (1.0, red)]);
        assert_ne!(
            a.intern_key(),
            b.intern_key(),
            "different gradient stops must not share a raster"
        );
    }
}
