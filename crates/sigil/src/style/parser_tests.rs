use crate::style::class_parse::parse_classes;
use crate::style::parse_class_string;

#[test]
fn classes_macro_parses_against_parse_class_string() {
    let expected = parse_class_string("p-16 bg-accent rounded-md").expect("parse");
    assert_eq!(
        parse_classes("p-16 bg-accent rounded-md"),
        expected.as_slice()
    );
}

#[test]
fn parse_classes_reuses_the_parsed_slice() {
    let input: &'static str = "p-16 bg-accent";
    let first = parse_classes(input);
    let second = parse_classes(input);
    assert!(core::ptr::eq(first, second));
}

#[cfg(debug_assertions)]
#[test]
fn parse_classes_cache_survives_a_diagnostic_panic() {
    let bad: &'static str = "cache-poison:x";
    drop(std::panic::catch_unwind(|| parse_classes(bad)));
    let input: &'static str = "p-12 bg-surface";
    assert!(core::ptr::eq(parse_classes(input), parse_classes(input)));
}
