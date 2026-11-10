use std::fs::File;
use std::io::BufWriter;
use std::panic::AssertUnwindSafe;
use std::path::Path;

use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, RgbaImage, imageops};
use serde_json::{Value, json};
use sigil_kittest::Error;

type Captured = (RgbaImage, Option<sigil::geometry::Rect>);
pub(crate) type Capture = Result<Option<Captured>, Error>;

#[derive(Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

pub(crate) struct Shots {
    pub(crate) images: usize,
    pub(crate) bytes: u64,
}

impl Shots {
    pub(crate) const fn new() -> Self {
        Self {
            images: 0,
            bytes: 0,
        }
    }

    pub(crate) fn pair(
        &mut self,
        out: &Path,
        stem: &str,
        crop_pad: u32,
        render: impl Fn(f32) -> Capture,
    ) -> Result<Value, String> {
        let base = rendered(&render, 1.0)?;
        let (rect, content) = content_rect(&base.0, crop_pad);
        let src = self.write(out, &format!("img/{stem}.webp"), &crop(&base.0, rect, 1))?;
        let wide = rendered(&render, 2.0)?;
        let src2x = self.write(out, &format!("img/{stem}@2x.webp"), &crop(&wide.0, rect, 2))?;
        let crop = json!({
            "x": rect.x,
            "y": rect.y,
            "canvas_width": base.0.width(),
            "canvas_height": base.0.height(),
            "pad": crop_pad,
        });
        Ok(json!({
            "src": src,
            "src2x": src2x,
            "width": rect.w,
            "height": rect.h,
            "crop": crop,
            "focus": focus(&base, rect, content),
        }))
    }

    fn write(&mut self, out: &Path, name: &str, image: &RgbaImage) -> Result<String, String> {
        let path = out.join(name);
        let file = File::create(&path).map_err(|err| format!("create {name}: {err}"))?;
        let (w, h) = image.dimensions();
        WebPEncoder::new_lossless(BufWriter::new(file))
            .encode(image.as_raw(), w, h, ExtendedColorType::Rgba8)
            .map_err(|err| format!("encode {name}: {err}"))?;
        let size = std::fs::metadata(&path).map_or(0, |meta| meta.len());
        self.images += 1;
        self.bytes += size;
        Ok(name.to_owned())
    }
}

fn focus(base: &Captured, rect: Rect, content: Rect) -> Value {
    let [left, top, right, bottom] = base.1.map_or_else(
        || {
            [
                content.x,
                content.y,
                content.x + content.w,
                content.y + content.h,
            ]
            .map(f64::from)
        },
        |layer| {
            [
                layer.origin.x,
                layer.origin.y,
                layer.origin.x + layer.width,
                layer.origin.y + layer.height,
            ]
            .map(f64::from)
        },
    );
    let clamp_x = |value: f64| (value - f64::from(rect.x)).clamp(0.0, f64::from(rect.w));
    let clamp_y = |value: f64| (value - f64::from(rect.y)).clamp(0.0, f64::from(rect.h));
    let (x, y) = (clamp_x(left).floor(), clamp_y(top).floor());
    json!({
        "x": x,
        "y": y,
        "w": clamp_x(right).ceil() - x,
        "h": clamp_y(bottom).ceil() - y,
    })
}

fn crop(image: &RgbaImage, rect: Rect, scale: u32) -> RgbaImage {
    let x = (rect.x * scale).min(image.width());
    let y = (rect.y * scale).min(image.height());
    let w = (rect.w * scale).min(image.width() - x);
    let h = (rect.h * scale).min(image.height() - y);
    imageops::crop_imm(image, x, y, w, h).to_image()
}

fn rendered(render: &impl Fn(f32) -> Capture, scale: f32) -> Result<Captured, String> {
    match std::panic::catch_unwind(AssertUnwindSafe(|| render(scale))) {
        Ok(Ok(Some(captured))) => Ok(captured),
        Ok(Ok(None)) => Err(String::from("render error: no wgpu adapter")),
        Ok(Err(err)) => Err(with_causes(format!("render error: {err}"), &err)),
        Err(_) => Err(String::from("render error: capture panicked")),
    }
}

fn with_causes(mut text: String, error: &dyn std::error::Error) -> String {
    let mut cause = error.source();
    while let Some(inner) = cause {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        cause = inner.source();
    }
    text
}

fn content_rect(image: &RgbaImage, pad: u32) -> (Rect, Rect) {
    let (width, height) = image.dimensions();
    let whole = Rect {
        x: 0,
        y: 0,
        w: width,
        h: height,
    };
    let Some(corner) = image.get_pixel_checked(0, 0).copied() else {
        return (whole, whole);
    };
    let mut bounds = [width, height, 0, 0];
    for (x, y, pixel) in image.enumerate_pixels() {
        if *pixel != corner {
            bounds = [
                bounds[0].min(x),
                bounds[1].min(y),
                bounds[2].max(x + 1),
                bounds[3].max(y + 1),
            ];
        }
    }
    let [left, top, right, bottom] = bounds;
    if right <= left || bottom <= top {
        return (whole, whole);
    }
    let content = Rect {
        x: left,
        y: top,
        w: right - left,
        h: bottom - top,
    };
    let x = left.saturating_sub(pad);
    let y = top.saturating_sub(pad);
    let rect = Rect {
        x,
        y,
        w: (right + pad).min(width) - x,
        h: (bottom + pad).min(height) - y,
    };
    (rect, content)
}
