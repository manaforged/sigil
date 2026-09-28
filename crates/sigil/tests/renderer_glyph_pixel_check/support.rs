use super::*;

pub(super) fn assert_sharp(label: &str, pixels: &[u8], w: u32, h: u32) {
    let measurement = measure(pixels, w, h);
    assert!(
        measurement.ink_fraction > 0.0,
        "{label}: ink_fraction was 0 — no glyph pixels rendered ({w}x{h})"
    );
    let ramp_px = measurement.softest_ramp_px.unwrap_or_else(|| {
        panic!(
            "{label}: no ink stem found; max_luminance={}",
            measurement.max_lum
        )
    });
    println!(
        "{label}: measured ramp_px={ramp_px:.3} (ink_fraction={:.4})",
        measurement.ink_fraction
    );
    assert!(
        ramp_px <= RAMP_THRESHOLD_PX,
        "{label}: AA ramp width {ramp_px:.3}px exceeds {RAMP_THRESHOLD_PX}px threshold \
         (ink_fraction={:.4}). Direct path should hold this at every DPI; a correctly \
         DPI-scaled embed path should too.",
        measurement.ink_fraction,
    );
}

pub(super) fn assert_soft(label: &str, pixels: &[u8], w: u32, h: u32) {
    let measurement = measure(pixels, w, h);
    assert!(
        measurement.ink_fraction > 0.0,
        "{label}: ink_fraction was 0 — no glyph pixels rendered ({w}x{h})"
    );
    let ramp_px = measurement.softest_ramp_px.unwrap_or_else(|| {
        panic!(
            "{label}: no ink stem found; max_luminance={}",
            measurement.max_lum
        )
    });
    println!(
        "{label}: measured ramp_px={ramp_px:.3} (ink_fraction={:.4})",
        measurement.ink_fraction
    );
    assert!(
        ramp_px > RAMP_THRESHOLD_PX,
        "{label}: AA ramp width {ramp_px:.3}px is within {RAMP_THRESHOLD_PX}px threshold \
         (ink_fraction={:.4}) — expected a mismatched scale to soften the ramp. If this now \
         passes as sharp, ScreenSubmission.scale stopped reaching the Slug draw.",
        measurement.ink_fraction,
    );
}

pub(super) fn rel_diff(a: f32, b: f32) -> f32 {
    let m = a.max(b);
    if m <= f32::EPSILON {
        0.0
    } else {
        (a - b).abs() / m
    }
}

pub(super) fn render_text(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_scale: f32,
    draw_scale: f32,
) -> (Vec<u8>, u32, u32) {
    render_text_inner(
        &RenderArgs {
            texture_scale,
            draw_scale,
            content: TEXT,
            text: sigil::hal::Color::WHITE,
            background: wgpu::Color::BLACK,
        },
        device,
        queue,
    )
}

pub(super) fn render_text_string(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scale: f32,
    content: &str,
    colors: (sigil::hal::Color, wgpu::Color),
) -> (Vec<u8>, u32, u32) {
    render_text_inner(
        &RenderArgs {
            texture_scale: scale,
            draw_scale: scale,
            content,
            text: colors.0,
            background: colors.1,
        },
        device,
        queue,
    )
}

pub(super) struct RenderArgs<'a> {
    pub(super) texture_scale: f32,
    pub(super) draw_scale: f32,
    pub(super) content: &'a str,
    pub(super) text: sigil::hal::Color,
    pub(super) background: wgpu::Color,
}

pub(super) fn render_text_inner(
    args: &RenderArgs<'_>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (Vec<u8>, u32, u32) {
    let &RenderArgs {
        texture_scale,
        draw_scale,
        content,
        text,
        background,
    } = args;
    let phys_w = (LOGICAL_W * texture_scale).round() as u32;
    let phys_h = (LOGICAL_H * texture_scale).round() as u32;

    let texture = create_capture_texture(device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let engine = OwnedTextEngine::new(TextConfig::default());
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: FONT_SIZE,
        font_weight: 400,
        line_height: TextLineHeight::FontSizeRelative(1.2),
        letter_spacing_em: 0.0,
        ..Default::default()
    };
    let paragraph = engine
        .layout_paragraph_styled(content, None, &style)
        .expect("layout_paragraph_styled");

    let mut pipeline = SlugTextPipeline::new(device, FORMAT);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("glyph-pixel-check encoder"),
    });
    clear_to(&mut encoder, &view, background);
    pipeline
        .record_paragraph(
            &mut encoder,
            &DrawParams {
                device,
                queue,
                view: &view,
                surface_width: LOGICAL_W,
                surface_height: LOGICAL_H,
                scale: draw_scale,
                paragraph: &paragraph,
                glyphs: None,
                origin: ORIGIN,
                transform: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                color: Some(text),
                clip: None,
            },
        )
        .expect("record_paragraph");
    queue.submit(core::iter::once(encoder.finish()));

    let pixels = read_pixels(device, queue, &texture, phys_w, phys_h).expect("sigil-kittest");
    (pixels, phys_w, phys_h)
}

pub(super) fn clear_to(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    color: wgpu::Color,
) {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("glyph-pixel-check clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

pub(super) struct Measurement {
    pub(super) max_lum: u8,
    pub(super) ink_fraction: f32,
    pub(super) ink_mass: f32,
    pub(super) softest_ramp_px: Option<f32>,
}

pub(super) fn measure(pixels: &[u8], width: u32, height: u32) -> Measurement {
    let max_lum = global_max_luminance(pixels, width, height);
    let max_f = f32::from(max_lum);
    let ink_fraction = ink_fraction(pixels, width, height, max_f);
    let ink_mass = ink_mass(pixels, width, height, max_f);
    let softest_ramp_px = (max_f >= 1.0)
        .then(|| softest_edge_ramp(pixels, width, height, max_f))
        .flatten();
    Measurement {
        max_lum,
        ink_fraction,
        ink_mass,
        softest_ramp_px,
    }
}

pub(super) fn ink_mass(pixels: &[u8], width: u32, height: u32, max_lum: f32) -> f32 {
    if max_lum < 1.0 {
        return 0.0;
    }
    let total = (width * height) as f32;
    let sum: f32 = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| f32::from(luminance(pixels, width, x, y)) / max_lum)
        .sum();
    sum / total
}

pub(super) fn luminance(pixels: &[u8], width: u32, x: u32, y: u32) -> u8 {
    pixels[((y * width + x) * BYTES_PER_PIXEL) as usize]
}
pub(super) fn global_max_luminance(pixels: &[u8], width: u32, height: u32) -> u8 {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| luminance(pixels, width, x, y))
        .max()
        .unwrap_or(0)
}
pub(super) fn ink_fraction(pixels: &[u8], width: u32, height: u32, max_lum: f32) -> f32 {
    let half = 0.5 * max_lum;
    let total = (width * height) as f32;
    let count = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .filter(|&(x, y)| f32::from(luminance(pixels, width, x, y)) > half)
        .count();
    count as f32 / total
}

pub(super) fn softest_edge_ramp(
    pixels: &[u8],
    width: u32,
    height: u32,
    max_lum: f32,
) -> Option<f32> {
    let lower = 0.10 * max_lum;
    let upper = 0.90 * max_lum;
    let mut worst: Option<f32> = None;
    for y in 0..height {
        let row: Vec<u8> = (0..width).map(|x| luminance(pixels, width, x, y)).collect();
        for run in solid_runs(&row, upper) {
            for (from, step) in [(run.0, -1), (run.1.saturating_sub(1), 1)] {
                let ramp_px = count_ramp(&row, from, step, lower, upper) as f32;
                if worst.is_none_or(|w| ramp_px > w) {
                    worst = Some(ramp_px);
                }
            }
        }
    }
    worst
}

pub(super) fn solid_runs(row: &[u8], upper: f32) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < row.len() {
        if f32::from(row[i]) >= upper {
            let start = i;
            while i < row.len() && f32::from(row[i]) >= upper {
                i += 1;
            }
            runs.push((start, i));
        } else {
            i += 1;
        }
    }
    runs
}

pub(super) fn count_ramp(row: &[u8], from: usize, step: i32, lower: f32, upper: f32) -> usize {
    let mut count = 0usize;
    let mut prev = f32::from(row[from]);
    let mut idx = from as i64 + i64::from(step);
    while idx >= 0 && (idx as usize) < row.len() {
        let v = f32::from(row[idx as usize]);
        if v > lower && v < upper && v < prev {
            count += 1;
            prev = v;
            idx += i64::from(step);
        } else {
            break;
        }
    }
    count
}
