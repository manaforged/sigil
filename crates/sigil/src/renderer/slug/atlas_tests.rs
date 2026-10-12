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

fn uploaded_layout() -> AtlasLayout {
    let mut layout = AtlasLayout::new();
    layout.offsets.insert(key(1), 0);
    layout.offsets.insert(key(2), 7);
    layout.last_generation = 7;
    layout
}

#[test]
fn current_generation_preserves_uploaded_offsets() {
    let mut layout = uploaded_layout();
    let before = layout.offsets.clone();
    assert!(matches!(
        layout.prepare_rebuild(2, 7),
        RebuildDecision::Current
    ));
    assert_eq!(layout.offsets, before);
}

#[test]
fn new_generation_keeps_uploaded_offsets() {
    let mut layout = uploaded_layout();
    layout.offsets.remove(&key(2));
    assert!(matches!(
        layout.prepare_rebuild(2, 8),
        RebuildDecision::Upload { .. }
    ));
    assert_eq!(layout.offsets.get(&key(1)), Some(&0));
    assert_eq!(layout.offsets.get(&key(2)), None);
}
