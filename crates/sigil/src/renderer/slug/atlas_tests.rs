use super::*;

fn key(glyph_id: u32) -> GlyphKey {
    GlyphKey {
        font_id: 42,
        font_index: 0,
        glyph_id,
        variation_coords: Box::new([]),
        skew_bits: 0,
    }
}

#[test]
fn current_generation_preserves_uploaded_offsets() {
    let first = key(1);
    let second = key(2);
    let mut layout = AtlasLayout::new();
    layout.record_offset(&first, 0);
    layout.record_offset(&second, BYTES_PER_GLYPH);
    layout.finish_upload(7);

    let first_offset = layout.offset(&first);
    let second_offset = layout.offset(&second);
    let decision = layout.prepare_rebuild(2, 7);

    assert!(matches!(decision, RebuildDecision::Current));
    assert_eq!(layout.offset(&first), first_offset);
    assert_eq!(layout.offset(&second), second_offset);
}

#[test]
fn new_generation_keeps_uploaded_offsets() {
    let first = key(1);
    let mut layout = AtlasLayout::new();
    layout.record_offset(&first, 0);
    layout.finish_upload(7);

    let decision = layout.prepare_rebuild(2, 8);

    assert!(matches!(decision, RebuildDecision::Upload { .. }));
    assert_eq!(layout.offset(&first), Some(0));
    assert_eq!(layout.offset(&key(2)), None);
}
