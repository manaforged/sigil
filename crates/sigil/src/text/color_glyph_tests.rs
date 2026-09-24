use super::*;

const EMOJI: &[u8] = include_bytes!("../../assets/text/fonts/NotoColorEmoji-subset.ttf");

#[test]
fn detects_color_bitmap_font() {
    let font = FontRef::new(EMOJI).unwrap();
    assert!(
        has_color_glyphs(&font),
        "Noto Color Emoji carries CBDT strikes"
    );
}

#[test]
fn rasterizes_check_mark_in_color() {
    let font = FontRef::new(EMOJI).unwrap();
    let gid = glyph_for_char(&font, '\u{2705}').expect("cmap has ✅");
    let raster = rasterize(&font, gid, 128.0, crate::hal::Color::default()).expect("color raster");

    assert!(
        raster.width > 8 && raster.height > 8,
        "plausible bitmap {}x{}",
        raster.width,
        raster.height
    );
    assert_eq!(
        raster.rgba.len(),
        (raster.width * raster.height * 4) as usize
    );

    let green_pixels = raster
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] > 200 && p[1] > 120 && p[1] > p[0] && p[1] > p[2])
        .count();
    assert!(
        green_pixels > 20,
        "check mark must contain opaque green pixels, found {green_pixels}"
    );
}

#[test]
fn non_emoji_glyph_has_no_color_raster() {
    let font = FontRef::new(EMOJI).unwrap();
    assert!(
        rasterize(
            &font,
            GlyphId::new(9_999),
            128.0,
            crate::hal::Color::default()
        )
        .is_none()
    );
}
