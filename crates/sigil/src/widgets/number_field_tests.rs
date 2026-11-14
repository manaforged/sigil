#![allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]

use super::private::Sealed as _;
use super::*;
use crate::reactive::{Owner, signal};
use crate::style::Locale;

#[test]
fn parse_and_clamp_int() {
    assert_eq!(i32::parse("42", &Locale::default()), Some(42));
    assert_eq!(i32::parse("not-a-number", &Locale::default()), None);
    assert_eq!(42i32.clamp_to(&(0..=10)), 10);
    assert_eq!((-5i32).clamp_to(&(0..=10)), 0);
}

#[test]
fn parse_and_clamp_float() {
    assert_eq!(f32::parse("3.5", &Locale::default()), Some(3.5));
    assert_eq!(f32::parse("inf", &Locale::default()), None);
    assert_eq!(f32::parse("NaN", &Locale::default()), None);
    assert_eq!(2.5f32.clamp_to(&(0.0..=1.0)), 1.0);
}

#[test]
fn building_leaves_the_value_signal_alone() {
    Owner::scope("number-field-resync", || {
        let inside = signal(50i32);
        let _inside = number_field(inside, 0..=100);
        assert_eq!(inside.peek(), 50);
        let outside = signal(150i32);
        let _outside = number_field(outside, 0..=100);
        assert_eq!(
            outside.peek(),
            150,
            "value signal itself is not mutated by display sync"
        );
    });
}

#[test]
fn unsigned_rejects_minus_and_stays_at_zero() {
    let locale = Locale::default();
    assert_eq!(u32::parse("-1", &locale), None);
    assert_eq!(u8::parse("-1", &locale), None);
    assert_eq!(u8::parse("300", &locale), None);
    assert_eq!(0u32.clamp_to(&(0..=10)), 0);
    assert_eq!(0u8.clamp_to(&(0..=10)), 0);
}

#[test]
fn wide_integers_round_trip() {
    let locale = Locale::default();
    assert_eq!(
        u64::parse(&u64::MAX.render(&locale), &locale),
        Some(u64::MAX)
    );
    assert_eq!(
        i128::parse(&i128::MIN.render(&locale), &locale),
        Some(i128::MIN)
    );
    assert_eq!(
        u128::parse(&u128::MAX.render(&locale), &locale),
        Some(u128::MAX)
    );
}

#[test]
fn every_builtin_number_type_builds_a_field() {
    Owner::scope("number-field-kinds", || {
        let _ = number_field(signal(0u8), 0..=10);
        let _ = number_field(signal(0u32), 0..=10);
        let _ = number_field(signal(0i128), -5..=5);
        let _ = number_field(signal(0isize), -5..=5);
    });
}
