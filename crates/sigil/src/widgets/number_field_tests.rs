#![allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]

use super::*;
use crate::reactive::Runtime;
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
    Runtime::scope("number-field-resync", || {
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
