use sigil::hal::{Color, PhysicalSize};
use sigil::renderer::{CpuRenderer, WgpuRenderer};
use sigil::scene::CommandScene;

use crate::Error;
use crate::gpu::{Capture, read_pixels, render_capture};

pub const CLOSE_TOLERANCE: u8 = 48;
pub const MAX_MEAN_DIFF: f32 = 20.0;
pub const MIN_CLOSE_FRACTION: f32 = 0.85;

pub struct Frame<'a> {
    pub scene: &'a CommandScene,
    pub viewport: (f32, f32),
    pub size: PhysicalSize,
    pub clear: Color,
}

impl Frame<'_> {
    #[allow(clippy::cast_precision_loss)]
    fn scale(&self) -> f32 {
        self.size.width as f32 / self.viewport.0.max(1.0)
    }
}

pub fn cpu_rgba(renderer: &mut CpuRenderer, frame: &Frame<'_>) -> Result<Vec<u8>, Error> {
    let pixmap = renderer
        .render(frame.scene, frame.clear, frame.viewport, frame.size)
        .map_err(|err| Error::Render(err.to_string()))?;
    let mut rgba = Vec::with_capacity(pixmap.pixels().len() * 4);
    for px in pixmap.pixels() {
        let c = px.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Ok(rgba)
}

pub fn gpu_rgba(renderer: &mut WgpuRenderer, frame: &Frame<'_>) -> Result<Vec<u8>, Error> {
    let device = renderer
        .device()
        .ok_or_else(|| Error::Device("renderer has no device".to_owned()))?
        .clone();
    let queue = renderer
        .queue()
        .ok_or_else(|| Error::Device("renderer has no queue".to_owned()))?
        .clone();
    let capture = Capture {
        viewport: frame.viewport,
        size_px: [frame.size.width, frame.size.height],
        scale: frame.scale(),
        clear: frame.clear,
    };
    let texture = render_capture(renderer, (&device, &queue), frame.scene, &capture)?;
    read_pixels(
        &device,
        &queue,
        &texture,
        frame.size.width,
        frame.size.height,
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Diff {
    pub mean: f32,
    pub close: f32,
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn diff(a: &[u8], b: &[u8]) -> Diff {
    let mut total = 0_u64;
    let mut close = 0_usize;
    let (a, _) = a.as_chunks::<4>();
    let (b, _) = b.as_chunks::<4>();
    let pixels = a.len().min(b.len()).max(1);
    for (pa, pb) in a.iter().zip(b) {
        let d = [
            pa[0].abs_diff(pb[0]),
            pa[1].abs_diff(pb[1]),
            pa[2].abs_diff(pb[2]),
        ];
        total += d.iter().map(|&v| u64::from(v)).sum::<u64>();
        if d.iter().all(|&v| v <= CLOSE_TOLERANCE) {
            close += 1;
        }
    }
    Diff {
        mean: total as f32 / (pixels * 3) as f32,
        close: close as f32 / pixels as f32,
    }
}

pub fn minimize(len: usize, mut fails: impl FnMut(&[usize]) -> bool) -> Vec<usize> {
    let mut kept: Vec<usize> = (0..len).collect();
    let mut chunk = kept.len().div_ceil(2).max(1);
    loop {
        let mut removed = false;
        let mut start = 0;
        while start < kept.len() {
            let end = (start + chunk).min(kept.len());
            let trial: Vec<usize> = kept
                .iter()
                .enumerate()
                .filter(|(at, _)| *at < start || *at >= end)
                .map(|(_, index)| *index)
                .collect();
            if fails(&trial) {
                kept = trial;
                removed = true;
            } else {
                start = end;
            }
        }
        if chunk == 1 && !removed {
            return kept;
        }
        chunk = if removed { chunk } else { chunk.div_ceil(2) }.min(kept.len().max(1));
    }
}

pub fn check_cpu(scene: &crate::scene_gen::GeneratedScene) -> Result<(), String> {
    let commands = scene.scene();
    let size = scene.physical();
    let render = |renderer: &mut CpuRenderer| {
        renderer
            .render(&commands, scene.clear, scene.viewport, size)
            .map_err(|err| format!("render failed: {err}"))
    };
    let mut renderer = CpuRenderer::new();
    let first = render(&mut renderer)?;
    let again = render(&mut renderer)?;
    let fresh = render(&mut CpuRenderer::new())?;
    if (first.width(), first.height()) != (size.width, size.height) {
        return Err(format!(
            "output {}x{} for target {}x{}",
            first.width(),
            first.height(),
            size.width,
            size.height
        ));
    }
    if let Some(px) = first
        .pixels()
        .iter()
        .find(|px| px.red() > px.alpha() || px.green() > px.alpha() || px.blue() > px.alpha())
    {
        return Err(format!("invalid premultiplied pixel {px:?}"));
    }
    if first.data() != again.data() {
        return Err("second render on the same renderer differs".to_owned());
    }
    if first.data() != fresh.data() {
        return Err("render on a fresh renderer differs".to_owned());
    }
    Ok(())
}
