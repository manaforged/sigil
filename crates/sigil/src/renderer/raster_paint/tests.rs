#![allow(clippy::unwrap_used, clippy::float_cmp, clippy::indexing_slicing)]

use super::*;

#[test]
fn subpixel_wrap_carries_into_anchor() {
    assert_eq!(subpixel_quantize(10.0), (0, 0.0));
    assert_eq!(subpixel_quantize(10.1), (0, 0.0));
    assert_eq!(subpixel_quantize(10.4), (2, 0.0));
    assert_eq!(subpixel_quantize(10.874), (3, 0.0));
    assert_eq!(subpixel_quantize(10.875), (0, 1.0));
    assert_eq!(subpixel_quantize(10.999), (0, 1.0));
}

#[test]
fn quantized_placement_error_bounded_by_half_bin() {
    let bins = f32::from(raster::SUBPIXEL_BINS);
    for i in 0..1000 {
        #[allow(clippy::cast_precision_loss)]
        let pen = 10.0 + (i as f32) / 1000.0;
        let (bin, carry) = subpixel_quantize(pen);
        let placed = (pen + carry).floor() + f32::from(bin) / bins;
        assert!(
            (placed - pen).abs() <= 0.5 / bins + 1e-4,
            "pen {pen}: placed {placed}, err {}",
            (placed - pen).abs()
        );
    }
}

#[test]
fn normalize_uvs_divides_by_final_size() {
    let mut instances = vec![
        RasterInstance {
            quad_rect: [0.0; 4],
            uv_rect: [0.0, 0.0, 512.0, 512.0],
            color: [0.0; 4],
        },
        RasterInstance {
            quad_rect: [0.0; 4],
            uv_rect: [1024.0, 0.0, 2048.0, 1024.0],
            color: [0.0; 4],
        },
    ];
    normalize_uvs(&mut instances, 2048);
    assert_eq!(instances[0].uv_rect, [0.0, 0.0, 0.25, 0.25]);
    assert_eq!(instances[1].uv_rect, [0.5, 0.0, 1.0, 0.5]);
}

const NOTO_HEBREW: &[u8] =
    include_bytes!("../../../tests/fixtures/text/NotoSansHebrew-Regular.ttf");

fn mixed_face_paragraph() -> Paragraph {
    let mut engine = crate::text::OwnedTextEngine::new(crate::text::TextConfig::default());
    engine
        .register_font_family("serif", NOTO_HEBREW.to_vec(), 0)
        .expect("NotoSansHebrew fixture must register");
    let style = crate::style::TextStyle {
        font_family: String::from("sans"),
        font_size: 16.0,
        font_weight: 400,
        line_height: crate::style::TextLineHeight::FontSizeRelative(1.2),
        ..crate::style::TextStyle::default()
    };
    engine
        .layout_paragraph_styled("Hi \u{05E9}\u{05DC}\u{05D5}\u{05DD}", None, &style)
        .expect("mixed-face paragraph must lay out")
}

fn try_headless_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let mut renderer = crate::renderer::WgpuRenderer::new().ok()?;
    renderer.ensure_headless_for_tests().ok()?;
    let device = renderer.device()?.clone();
    let queue = renderer.queue()?.clone();
    Some((device, queue))
}

fn capture_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("raster-paint-test capture"),
        size: wgpu::Extent3d {
            width: 128,
            height: 48,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn find_fallback_glyph<'a>(
    paragraph: &Paragraph,
    glyphs: &'a [RenderedGlyph],
) -> &'a RenderedGlyph {
    let mut fonts_seen: Vec<u64> = Vec::new();
    paragraph.for_each_font(|font_id, _bytes, _index| fonts_seen.push(font_id));
    assert!(
        fonts_seen.len() >= 2,
        "expected primary + fallback faces in this paragraph, got {fonts_seen:?}",
    );
    let primary_font_id = fonts_seen[0];
    glyphs
        .iter()
        .find(|g| g.font_id != primary_font_id)
        .expect("expected at least one fallback-face glyph")
}

#[allow(clippy::print_stdout)]
#[test]
fn fallback_face_glyph_lands_in_atlas_not_leftover() {
    let Some((device, queue)) = try_headless_device() else {
        println!("no adapter — skipping fallback_face_glyph_lands_in_atlas_not_leftover");
        return;
    };

    let paragraph = mixed_face_paragraph();
    let glyphs = paragraph.rendered_glyphs();
    let fallback_glyph = find_fallback_glyph(&paragraph, &glyphs);

    let mut pipeline = RasterTextPipeline::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let texture = capture_texture(&device);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("raster-paint-test encoder"),
    });
    let params = RasterDrawParams {
        device: &device,
        queue: &queue,
        view: &view,
        surface_width: 128.0,
        surface_height: 48.0,
        scale: 1.0,
        paragraph: &paragraph,
        glyphs: &glyphs,
        origin: (0.0, 0.0),
        color: Color::WHITE,
        clip: None,
    };
    let mut encoder = crate::renderer::split_encoder::SplitEncoder::new(&device, &queue, encoder);
    let leftover = pipeline.record_paragraph(&mut encoder, &params);
    queue.submit(core::iter::once(encoder.into_inner().finish()));

    assert!(
        !leftover
            .iter()
            .any(|g| g.font_id == fallback_glyph.font_id && g.glyph_id == fallback_glyph.glyph_id),
        "a fallback-face glyph must not be routed to leftover when the atlas has room",
    );

    let physical_pen_x = (params.origin.0 + fallback_glyph.x) * params.scale;
    let (bin, _carry) = subpixel_quantize(physical_pen_x);
    let key = RasterKey {
        font_id: fallback_glyph.font_id,
        font_index: fallback_glyph.font_index,
        glyph_id: fallback_glyph.glyph_id,
        physical_px_bits: (fallback_glyph.size * params.scale).to_bits(),
        bin,
    };
    assert!(
        pipeline.atlas.get(&key).is_some(),
        "the fallback-face glyph must be present in the raster atlas cache",
    );
}
