#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::collection::FontCollection;
use super::{
    Font, PushRunArgs, Script, Style, push_run, run_has_disallowed_notdef, shape_validated,
};
use crate::text::glyph::GlyphId;

const FIRA_REGULAR: &[u8] = sigil_fonts::FIRA_SANS_REGULAR;
const NOTO_HEBREW: &[u8] =
    include_bytes!("../../../tests/fixtures/text/NotoSansHebrew-Regular.ttf");

fn fira_regular() -> Font {
    Font::from_bytes(FIRA_REGULAR.to_vec(), 0).expect("FiraSans-Regular fixture must parse")
}

fn noto_hebrew() -> Font {
    Font::from_bytes(NOTO_HEBREW.to_vec(), 0).expect("NotoSansHebrew fixture must parse")
}

fn args(fragment: &str) -> PushRunArgs<'_> {
    PushRunArgs {
        fragment,
        start_byte: 0,
        level: 0,
        script: Script::Common,
        is_break_opportunity: false,
    }
}

#[test]
fn split_coverage_segment_renders_fully_via_two_faces() {
    let chain = vec![fira_regular(), noto_hebrew()];
    let fragment = "a\u{05D0}";
    let mut out = Vec::new();
    push_run(&args(fragment), &chain, &Style::regular(16.0), &mut out);

    assert_eq!(
        out.len(),
        2,
        "a two-cluster, two-face fragment must coalesce into exactly 2 sub-runs",
    );
    assert_eq!(
        out[0].face_slot, 0,
        "'a' must shape with the primary face (slot 0)"
    );
    assert_eq!(
        out[1].face_slot, 1,
        "Alef must fall back to the covering face (slot 1)",
    );
    for seg in &out {
        assert!(
            seg.run.glyphs.iter().all(|g| g.id != GlyphId(0)),
            "every sub-run must be glyph-covered, no .notdef tofu: {:?}",
            seg.run.glyphs.iter().map(|g| g.id).collect::<Vec<_>>(),
        );
    }
}

#[test]
fn primary_preference_never_swaps_when_primary_covers() {
    let noto = noto_hebrew();
    assert!(
        noto.covers("abc"),
        "test precondition: NotoSansHebrew must also cover plain Latin \
         for this to be a meaningful primary-preference check",
    );
    let chain = vec![fira_regular(), noto];
    let mut out = Vec::new();
    push_run(&args("abc"), &chain, &Style::regular(16.0), &mut out);

    assert_eq!(out.len(), 1, "fully-primary-covered text must not split");
    assert_eq!(
        out[0].face_slot, 0,
        "primary face must win even though the fallback also covers 'abc'",
    );
}

#[test]
fn shape_validated_falls_through_when_the_initial_pick_shapes_to_notdef() {
    let chain = vec![fira_regular(), noto_hebrew()];
    let (slot, run) = shape_validated(
        &chain,
        0,
        "\u{05D0}\u{05D1}\u{05D2}",
        1,
        &Style::regular(16.0),
    );

    assert_eq!(
        slot, 1,
        "shape_validated must fall through past the notdef-producing initial pick",
    );
    assert!(
        run.glyphs.iter().all(|g| g.id != GlyphId(0)),
        "the recovered shape must be glyph-covered: {:?}",
        run.glyphs.iter().map(|g| g.id).collect::<Vec<_>>(),
    );
}

#[test]
fn shape_validated_keeps_the_initial_pick_when_it_already_shapes_cleanly() {
    let chain = vec![fira_regular(), noto_hebrew()];
    let (slot, run) = shape_validated(&chain, 0, "abc", 0, &Style::regular(16.0));
    assert_eq!(slot, 0);
    assert!(run.glyphs.iter().all(|g| g.id != GlyphId(0)));
}

fn assert_ignorable_control_stays_bounded(control: char) {
    let fragment = format!("a{control}b");
    let chain = vec![fira_regular(), noto_hebrew()];
    let (slot, run) = shape_validated(&chain, 0, &fragment, 0, &Style::regular(16.0));
    assert_eq!(
        slot, 0,
        "an ignorable control must not push shape_validated past the \
         primary face (fragment {fragment:?})",
    );
    assert!(
        !run_has_disallowed_notdef(&run, &fragment),
        "an ignorable control's own .notdef must not count as a visible \
         coverage failure (fragment {fragment:?}): {:?}",
        run.glyphs.iter().map(|g| g.id).collect::<Vec<_>>(),
    );
}

#[test]
fn shape_validated_bounded_for_combining_grapheme_joiner() {
    assert_ignorable_control_stays_bounded('\u{034F}');
}

#[test]
fn shape_validated_bounded_for_word_joiner() {
    assert_ignorable_control_stays_bounded('\u{2060}');
}

#[test]
fn shape_validated_bounded_for_supplementary_variation_selector() {
    assert_ignorable_control_stays_bounded('\u{E0100}');
}

#[test]
fn registered_fallback_face_joins_the_chain() {
    let mut collection = super::FontCollection::with_default();
    let hebrew = noto_hebrew();
    collection.fallbacks = vec![hebrew.clone()];
    let primary = collection.primary().clone();
    let chain = collection.fallback_chain(&primary, 400, "");
    assert!(
        chain.iter().any(|f| f.id() == hebrew.id()),
        "fallback face must appear in the chain",
    );
    let emoji_last = chain.last().expect("chain is never empty");
    assert_ne!(
        emoji_last.id(),
        hebrew.id(),
        "emoji stays the last resort, after registered fallbacks",
    );
}

#[test]
fn bundled_default_family_resolves_weight_500_to_medium() {
    let collection = FontCollection::with_default();
    let regular = collection.select_weight("sans", 400);
    let medium = collection.select_weight("sans", 500);
    assert_ne!(
        regular.id(),
        medium.id(),
        "bundled sans family must contain a distinct Medium face"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn mixed_script_names_shape_without_notdef() {
    let engine = crate::text::OwnedTextEngine::new(crate::text::TextConfig::default());
    for text in [
        "KW.M · 東京証券 株式会社",
        "مصرف الخليج الأول",
        "בנק הפועלים",
        "서울반도체",
    ] {
        let para = engine
            .layout_paragraph_styled(text, None, &crate::style::TextStyle::default())
            .expect("layout");
        let notdef = para
            .rendered_glyphs()
            .iter()
            .filter(|g| g.glyph_id == 0)
            .count();
        assert_eq!(notdef, 0, "{text} shaped {notdef} notdef glyphs");
    }
}
