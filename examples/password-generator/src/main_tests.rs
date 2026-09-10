#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn tree_builds_without_panic() {
    with_cx("password-generator-smoke", |cx| {
        let root = app_tree(cx);
        let mut count = 0_usize;
        cx.visit_preorder(root, |_, _| count += 1);
        assert!(
            count >= 8,
            "expected at least 8 nodes (root + readout + 4 toggles + slider + button), got {count}",
        );
    });
}

#[test]
fn generate_respects_length_and_alphabet() {
    let spec = PasswordSpec {
        length: 32,
        classes: CLASS_LOWER,
    };
    let pw = generate(spec, 0xDEAD_BEEF_CAFE_0001);
    assert_eq!(pw.len(), 32);
    assert!(
        pw.chars().all(|c| c.is_ascii_lowercase()),
        "lowercase-only set yielded non-lowercase char: {pw}",
    );
}

#[test]
fn generate_includes_all_classes_when_enabled() {
    let spec = PasswordSpec {
        length: 128,
        classes: CLASS_LOWER | CLASS_UPPER | CLASS_DIGITS | CLASS_SYMBOLS,
    };
    let pw = generate(spec, 0xA5A5_A5A5_A5A5_A5A5);
    assert!(pw.chars().any(|c| c.is_ascii_lowercase()));
    assert!(pw.chars().any(|c| c.is_ascii_uppercase()));
    assert!(pw.chars().any(|c| c.is_ascii_digit()));
    assert!(pw.chars().any(|c| SYMBOLS.contains(&(c as u8))));
}

#[test]
fn empty_alphabet_returns_hint() {
    let spec = PasswordSpec {
        length: 16,
        classes: 0,
    };
    let pw = generate(spec, 1);
    assert!(pw.contains("pick at least one"));
}

#[test]
fn xorshift_changes_state() {
    let a = xorshift64(1);
    let b = xorshift64(a);
    assert_ne!(a, 1);
    assert_ne!(a, b);
}
