//! White-box unit tests for the per-run font fallback chain
//! (`select_face` / `push_run` / `coalesce_face_spans` /
//! `shape_validated`) — adversarial review findings 1 and 2.
//!
//! These call `push_run`/`shape_validated` directly with a hand-built
//! fragment rather than going through the public `layout_paragraph`
//! path, because a genuinely mixed-script fragment (e.g. Latin +
//! Hebrew) is already split into separate segments upstream by
//! [`same_run`]'s script-boundary check before it would ever reach
//! `push_run` in the real pipeline — this exercises the fallback
//! mechanism itself in isolation, the same way `shaper/tests.rs` and
//! `shaping_complex/tests.rs` test their layer in isolation.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::{
    Font, PushRunArgs, Script, Style, push_run, run_has_disallowed_notdef, shape_validated,
};
use crate::glyph::GlyphId;

const FIRA_REGULAR: &[u8] = include_bytes!("../../assets/fonts/FiraSans-Regular.ttf");
const NOTO_HEBREW: &[u8] = include_bytes!("../../tests/fixtures/NotoSansHebrew-Regular.ttf");

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
    // "aא" — Latin 'a' immediately followed by Hebrew Alef, as ONE
    // fragment (finding 1: a segment whose coverage is divided between
    // faces must still render every cluster, not tofu the whole run
    // because no single face covers it end to end).
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
    // Both faces cover plain Latin "abc" (Noto Sans Hebrew ships full
    // Latin coverage) — the primary (slot 0) must always win when it
    // covers, proving `select_face`'s chain-order preference isn't
    // just "whichever face happens to be the only option."
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
    // Finding 2: cmap presence alone doesn't prove shaping success.
    // Simulate a wrong initial pick (slot 0 = FiraSans, which has no
    // Hebrew coverage) shaping Hebrew text — `shape_validated` must
    // detect the resulting `.notdef` and retry the rest of the chain,
    // landing on the face that actually shapes it (slot 1).
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
    // The common case: no fallthrough needed, no wasted extra shapes
    // beyond the one validation pass.
    let chain = vec![fira_regular(), noto_hebrew()];
    let (slot, run) = shape_validated(&chain, 0, "abc", 0, &Style::regular(16.0));
    assert_eq!(slot, 0);
    assert!(run.glyphs.iter().all(|g| g.id != GlyphId(0)));
}

/// Review follow-up A: assert a default-ignorable control embedded
/// in otherwise-primary-covered text neither surfaces a visible
/// `.notdef` from `shape_validated`'s own check, nor pushes it past
/// the primary face. NOTE (verified via a direct glyph-buffer probe):
/// `HarfRust` substitutes a real, non-`.notdef` placeholder glyph for
/// CGJ/WJ/VS17 rather than emitting `.notdef` for them at all in this
/// fixture pairing, so this specific assertion pair holds under BOTH
/// the pre-fix 4-char hardcoded classifier and the new range-table one
/// — it is not, by itself, a discriminating regression test for this
/// particular fixture/shaper combination. The discriminating tests for
/// finding A are the `covers_treats_*_as_transparent` tests in
/// `shaper/covers_tests.rs`, verified via a raw charmap probe showing
/// `FiraSans`'s cmap genuinely lacks CGJ/WJ/VS17 entries — `covers()`
/// returning `false` for them under the old classifier is confirmed,
/// not assumed. This test is kept because it is still a real,
/// non-trivial assertion about current `shape_validated` behavior for
/// these inputs (it would fail if a future change broke the
/// no-extra-attempts contract), just not proof of the classifier fix
/// specifically.
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
    assert_ignorable_control_stays_bounded('\u{034F}'); // CGJ
}

#[test]
fn shape_validated_bounded_for_word_joiner() {
    assert_ignorable_control_stays_bounded('\u{2060}'); // WORD JOINER
}

#[test]
fn shape_validated_bounded_for_supplementary_variation_selector() {
    assert_ignorable_control_stays_bounded('\u{E0100}'); // VS17
}

/// A face registered in `DefaultFonts::fallbacks` joins the per-run
/// chain: Hebrew shapes through it without smuggling the face into a
/// named family slot.
#[test]
fn registered_fallback_face_joins_the_chain() {
    let mut collection = super::FontCollection::with_default();
    let hebrew = noto_hebrew();
    collection.fallbacks = vec![hebrew.clone()];
    let primary = collection.primary().clone();
    let chain = collection.fallback_chain(&primary, 400);
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
