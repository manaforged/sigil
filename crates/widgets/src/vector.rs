use std::cell::RefCell;
use std::collections::HashMap;

use sigil_kernel::{Cx, ImageSource, IntoNode, NodeId, container, image};
use sigil_layout::prelude::LayoutStyle;
use sigil_style::Rgba8;
use tiny_skia::{
    Color as TsColor, FillRule, GradientStop, LinearGradient, Paint, PathBuilder as TsPathBuilder,
    Pixmap, Point as TsPoint, RadialGradient, Shader, SpreadMode, Stroke, Transform,
};

pub enum PathFill {
    Solid(Rgba8),
    Linear {
        start: (f32, f32),
        end: (f32, f32),
        stops: Vec<(f32, Rgba8)>,
    },
    Radial {
        center: (f32, f32),
        radius: f32,
        stops: Vec<(f32, Rgba8)>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCmd {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32),
    Close,
}

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
    #[must_use]
    pub fn move_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::MoveTo(x, y));
        self
    }

    #[must_use]
    pub fn line_to(mut self, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::LineTo(x, y));
        self
    }

    #[must_use]
    pub fn quad_to(mut self, cx: f32, cy: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::QuadTo(cx, cy, x, y));
        self
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the argument list mirrors the upstream API contract"
    )]
    #[must_use]
    pub fn cubic_to(mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> Self {
        self.cmds.push(PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y));
        self
    }

    #[must_use]
    pub fn close(mut self) -> Self {
        self.cmds.push(PathCmd::Close);
        self
    }

    #[must_use]
    pub fn fill(mut self, color: Rgba8) -> Self {
        self.fill = Some(PathFill::Solid(color));
        self
    }

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

    #[must_use]
    pub const fn stroke(mut self, color: Rgba8, width: f32) -> Self {
        self.stroke = Some((color, width));
        self
    }

    #[must_use]
    pub fn layout(mut self, style: LayoutStyle) -> Self {
        self.layout = Some(style);
        self
    }

    #[must_use]
    pub fn fit(self) -> Self {
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let logical = self.width as f32;
        self.fit_size(logical)
    }

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

    fn rasterize(&self) -> Option<ImageSource> {
        let key = self.intern_key();
        if let Some(hit) = RASTER.with(|map| map.borrow().get(&key).cloned()) {
            return Some(hit);
        }
        sigil_profile::count("path.raster");
        let path = build_ts_path(&self.cmds)?;
        let scale = f32::from(self.scale.max(1));
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let pix_w = ((self.width as f32) * scale).round() as u32;
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
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
            // `stroke_path` widens in path coordinates and the transform
            // scales the outline, so the width must stay in path units.
            let stroke = Stroke {
                width,
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
            None => container().build(cx),
        }
    }
}

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
                0.0,
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
#[path = "vector_tests.rs"]
mod tests;
