use super::*;

#[test]
fn line_aware_l1_resets_trailing_neutral_that_paragraph_wide_levels_miss() {
    let text = "\u{05D0}\u{05D1}\u{05D2} abc def";
    let space_before_def = 10_usize;
    let cut = 11_usize;

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
        line_aware.as_slice(),
        paragraph_wide_order.as_slice(),
        "line-aware L1 reorder must differ from the paragraph-wide-levels \
         reorder at this wrap point — got {line_aware:?} for both",
    );
    assert_eq!(
        line_aware.first().copied(),
        Some(3),
        "expected the trailing space (logical index 3) to lead the \
         visually-reordered line, got {line_aware:?}",
    );
}

#[test]
fn latin_and_numeric_input_is_all_zero() {
    for input in ["hello world", "123 456", "caf\u{00E9}"] {
        let levels = paragraph_levels(input);
        assert_eq!(levels.len(), input.len());
        assert!(levels.iter().all(|&l| l == 0), "{input} must be level 0");
    }
}

#[test]
fn empty_input_empty_levels() {
    let levels = paragraph_levels("");
    assert!(levels.is_empty());
}

#[test]
fn hebrew_only_rtl_levels() {
    let input = "\u{05D0}\u{05D1}\u{05D2}";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    assert!(
        levels.iter().all(|&l| l % 2 == 1),
        "Hebrew paragraph must be odd-leveled, got {levels:?}",
    );
}

#[test]
fn mixed_latin_hebrew_alternates() {
    let input = "abc\u{05D0}\u{05D1}";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    assert_eq!(&levels[0..3], &[0, 0, 0]);
    for (i, &l) in levels.iter().enumerate().skip(3) {
        assert!(l % 2 == 1, "byte {i} expected odd level, got {l}");
    }
}

#[test]
fn arabic_with_ascii_neutrals() {
    let input = "\u{0627} 12 \u{0628}";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    let alef_len = '\u{0627}'.len_utf8();
    for i in 0..alef_len {
        assert!(levels[i] % 2 == 1, "Arabic alef byte {i} must be odd");
    }
    let digit_start = alef_len + 1;
    assert_eq!(
        levels[digit_start], 2,
        "ASCII digit inside RTL paragraph should be level 2",
    );
}

#[test]
fn lrm_mark_does_not_panic() {
    let input = "ab\u{200E}cd";
    let levels = paragraph_levels(input);
    assert_eq!(levels.len(), input.len());
    assert_eq!(levels[0], 0);
    assert_eq!(levels[1], 0);
}
