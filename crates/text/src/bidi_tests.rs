use super::*;

#[test]
fn line_aware_l1_resets_trailing_neutral_that_paragraph_wide_levels_miss() {
    // "אבג abc def" (RTL Hebrew word, two LTR Latin words) inside a
    // pure-RTL-base paragraph. The space BETWEEN "abc" and "def"
    // has a LATIN neighbour on both sides, so `unicode_bidi`'s
    // ordinary (whole-paragraph) neutral resolution matches it to
    // the Latin run's level (2) — correct IF the line keeps
    // running into "def". But if the line is cut right after that
    // space (as if "def" wrapped away), UAX #9 rule L1 requires
    // resetting it to the PARAGRAPH level (1) instead, since it is
    // now trailing whitespace at the line's own end. Paragraph-wide
    // levels alone (pre-Wave-4-fix behaviour) can't see that the
    // line was cut here — this is the exact gap `visual_order_for_line`
    // exists to close.
    let text = "\u{05D0}\u{05D1}\u{05D2} abc def";
    let space_before_def = 10_usize; // byte offset of the space between "abc" and "def"
    let cut = 11_usize; // right after that space — "def" wraps away

    let paragraph_wide = paragraph_levels(text);
    assert_eq!(
        paragraph_wide[space_before_def], 2,
        "sanity: whole-paragraph resolution matches the space to the \
         Latin run (level 2), got {paragraph_wide:?}",
    );

    let pb = ParagraphBidi::new(text);
    let info = pb
        .info
        .as_ref()
        .expect("mixed-script text takes the full bidi path");
    let line_adjusted = info.reordered_levels(0..cut);
    assert_eq!(
        line_adjusted[space_before_def].number(),
        1,
        "line-aware L1 must reset the line-final space to the paragraph \
         level (1), not the whole-paragraph match-neighbour level (2)",
    );
}

#[test]
fn visual_order_for_line_diverges_from_paragraph_wide_reorder_at_a_wrap_point() {
    // Same scenario as above, exercised through the public
    // `ParagraphBidi` API: reordering the line
    // [hebrew_word, space, "abc", space] with line-aware L1 must
    // produce a DIFFERENT permutation than feeding the raw
    // paragraph-wide levels straight into `visual_order` — proving
    // the line-aware path is not a no-op wrapper.
    let text = "\u{05D0}\u{05D1}\u{05D2} abc def";
    let hebrew_end = 6_u32;
    let space1_end = 7_u32;
    let abc_end = 10_u32;
    let space2_end = 11_u32;
    let segment_ends = [hebrew_end, space1_end, abc_end, space2_end];

    let pb = ParagraphBidi::new(text);
    let line_aware = pb.visual_order_for_line(0, space2_end as usize, &segment_ends);

    let paragraph_wide_levels = paragraph_levels(text);
    let raw_levels: Vec<u8> = segment_ends
        .iter()
        .map(|&end| paragraph_wide_levels[end as usize - 1])
        .collect();
    let paragraph_wide_order = visual_order(&raw_levels);

    assert_ne!(
        line_aware, paragraph_wide_order,
        "line-aware L1 reorder must differ from the paragraph-wide-levels \
         reorder at this wrap point — got {line_aware:?} for both",
    );
    // The line-aware result reflects the L1-corrected levels
    // [1, 1, 2, 1]: only "abc" (index 2) sits at level 2, so L2's
    // "reverse contiguous >= max_level" pass touches just that one
    // element (no-op on a length-1 run), then the whole line
    // reverses at level 1 — the trailing space (index 3) ends up
    // FIRST (visually leftmost), not "abc" (index 2).
    assert_eq!(
        line_aware.first().copied(),
        Some(3),
        "expected the trailing space (logical index 3) to lead the \
         visually-reordered line, got {line_aware:?}",
    );
}

#[test]
fn latin_only_all_zero() {
    let input = "hello world";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    assert!(levels.iter().all(|&l| l == 0), "Latin must be level 0");
}

#[test]
fn empty_input_empty_levels() {
    let levels = paragraph_levels("");
    assert!(levels.is_empty());
}

#[test]
fn hebrew_only_rtl_levels() {
    // Hebrew letters Alef, Bet, Gimel.
    let input = "\u{05D0}\u{05D1}\u{05D2}";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    // Hebrew paragraph base direction is RTL; every byte sits at
    // an odd level (1 for a pure-Hebrew paragraph).
    assert!(
        levels.iter().all(|&l| l % 2 == 1),
        "Hebrew paragraph must be odd-leveled, got {levels:?}",
    );
}

#[test]
fn mixed_latin_hebrew_alternates() {
    // "abc" + Alef + Bet
    let input = "abc\u{05D0}\u{05D1}";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    // First three bytes are ASCII -> level 0 (paragraph base is LTR
    // because the first strong character is Latin).
    assert_eq!(&levels[0..3], &[0, 0, 0]);
    // The Hebrew bytes sit at an odd level (>=1).
    for (i, &l) in levels.iter().enumerate().skip(3) {
        assert!(l % 2 == 1, "byte {i} expected odd level, got {l}");
    }
}

#[test]
fn arabic_with_ascii_neutrals() {
    // Arabic Alef + space + ASCII digits + space + Arabic Beh.
    let input = "\u{0627} 12 \u{0628}";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    // Paragraph base direction is RTL (first strong char is Arabic).
    // Arabic letter bytes are at the odd RTL level.
    let alef_len = '\u{0627}'.len_utf8();
    for i in 0..alef_len {
        assert!(levels[i] % 2 == 1, "Arabic alef byte {i} must be odd");
    }
    // The ASCII digits get an EN (European Number) level — even
    // (level 2) inside an RTL paragraph per UAX #9.
    let digit_start = alef_len + 1; // skip space
    assert_eq!(
        levels[digit_start], 2,
        "ASCII digit inside RTL paragraph should be level 2",
    );
}

#[test]
fn all_numeric_is_zero() {
    let input = "123 456";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    assert!(
        levels.iter().all(|&l| l == 0),
        "ASCII numerics must be level 0 (LTR paragraph)",
    );
}

#[test]
fn lrm_mark_does_not_panic() {
    // U+200E LEFT-TO-RIGHT MARK — explicit bidi formatting char.
    // Must route through the full algorithm without crashing.
    let input = "ab\u{200E}cd";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    // ASCII bytes stay at level 0.
    assert_eq!(levels[0], 0);
    assert_eq!(levels[1], 0);
}

#[test]
fn latin_extended_stays_fast_path() {
    // 'é' as U+00E9 (NFC) is Latin-1 Supplement — fast path.
    let input = "caf\u{00E9}";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    assert!(levels.iter().all(|&l| l == 0));
}
