#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

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
    let bins = f32::from(crate::renderer::raster::SUBPIXEL_BINS);
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

fn key_at_unit_scale(glyph: &RenderedGlyph) -> RasterKey {
    let (bin, _) = subpixel_quantize(glyph.x);
    RasterKey::new(glyph, glyph.size, bin)
}

#[allow(clippy::print_stdout)]
#[test]
fn one_placement_caches_every_glyph_including_fallback_faces() {
    let Some((device, queue)) = crate::renderer::WgpuRenderer::new().ok().and_then(|mut r| {
        r.ensure_headless_for_tests().ok()?;
        Some((r.device()?.clone(), r.queue()?.clone()))
    }) else {
        println!("no adapter — skipping one_placement_caches_every_glyph_including_fallback_faces");
        return;
    };
    let paragraph = mixed_face_paragraph();
    let glyphs = paragraph.rendered_glyphs();
    let mut fonts_seen: Vec<u64> = Vec::new();
    paragraph.for_each_font(|font_id, _bytes, _index| fonts_seen.push(font_id));
    assert!(
        fonts_seen.len() >= 2,
        "expected primary + fallback faces in this paragraph, got {fonts_seen:?}",
    );
    let fallback = glyphs
        .iter()
        .find(|g| g.font_id != fonts_seen[0])
        .expect("expected at least one fallback-face glyph");
    let mut pipeline = RasterTextPipeline::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let (_, leftover) = pipeline.place_paragraph(&RasterDrawParams {
        device: &device,
        queue: &queue,
        scale: 1.0,
        paragraph: &paragraph,
        glyphs: &glyphs,
        origin: (0.0, 0.0),
        color: Color::WHITE,
    });
    assert!(
        !leftover
            .iter()
            .any(|g| g.font_id == fallback.font_id && g.glyph_id == fallback.glyph_id),
        "a fallback-face glyph must not be routed to leftover when the atlas has room",
    );
    assert!(
        pipeline.atlas.get(&key_at_unit_scale(fallback)).is_some(),
        "the fallback-face glyph must be present in the raster atlas cache",
    );
    let unknown: Vec<u32> = glyphs
        .iter()
        .filter(|glyph| {
            let key = key_at_unit_scale(glyph);
            pipeline.atlas.get(&key).is_none() && !pipeline.atlas.blank.contains(&key)
        })
        .map(|glyph| glyph.glyph_id)
        .collect();
    assert!(
        unknown.is_empty(),
        "glyphs {unknown:?} would be rasterized again on the next frame"
    );
}
