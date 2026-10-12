use super::*;

const FIRA: &[u8] = sigil_fonts::FIRA_SANS_REGULAR;

fn mask(glyph_id: u32, physical_px: f32, bin: u8) -> Option<GlyphMask> {
    Hinters::default().rasterize(
        FIRA,
        &RasterKey {
            font_id: 1,
            font_index: 0,
            glyph_id,
            physical_px_bits: physical_px.to_bits(),
            bin,
            coords: 0,
        },
        &[],
        0.0,
    )
}

fn gid(ch: char) -> u32 {
    FontRef::new(FIRA)
        .unwrap()
        .charmap()
        .map(ch)
        .unwrap()
        .to_u32()
}

#[test]
fn small_glyph_has_ink_and_sane_dims() {
    let a = gid('A');
    let mask = mask(a, 12.0, 0).unwrap();
    assert!(mask.width > 0 && mask.width <= 16, "width {}", mask.width);
    assert!(
        mask.height > 0 && mask.height <= 16,
        "height {}",
        mask.height
    );
    assert_eq!(mask.data.len(), (mask.width * mask.height) as usize);
    assert!(mask.data.iter().any(|&b| b > 128), "expected solid ink");
    assert!(
        mask.top > 0,
        "cap-height glyph rises above baseline, got top {}",
        mask.top
    );
}

#[test]
fn mask_is_not_vertically_flipped() {
    let mask = mask(gid('L'), 16.0, 0).unwrap();
    let w = mask.width as usize;
    let row_ink = |r: usize| -> u32 {
        mask.data[r * w..(r + 1) * w]
            .iter()
            .map(|&b| u32::from(b))
            .sum()
    };
    let top = row_ink(1);
    let bottom = row_ink(mask.height as usize - 2);
    assert!(
        bottom > top,
        "'L' foot must be at the bottom row (top ink {top}, bottom ink {bottom})"
    );
}

#[test]
fn subpixel_bins_shift_coverage() {
    let a = gid('A');
    let bin0 = mask(a, 12.0, 0).unwrap();
    let bin2 = mask(a, 12.0, 2).unwrap();
    assert!(
        bin0.width != bin2.width || bin0.data != bin2.data,
        "bin 0 and bin 2 rendered identical masks"
    );
}

#[test]
fn physical_size_scales_mask() {
    let a = gid('A');
    let s100 = mask(a, 12.0, 0).unwrap();
    let s200 = mask(a, 24.0, 0).unwrap();
    let ratio = f64::from(s200.height) / f64::from(s100.height);
    assert!(
        (1.6..=2.4).contains(&ratio),
        "24px mask should be ~2x the 12px mask, got {ratio}"
    );
}

#[test]
fn outline_less_glyph_returns_none() {
    assert!(mask(gid(' '), 12.0, 0).is_none());
}
