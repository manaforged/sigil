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

#[test]
fn tailwind_gradient_and_border_style_classes_parse_to_paint_atoms() {
    use crate::style::atom::{BorderStyle, GradientShape, PaintAtom, StopSlot};
    use crate::style::{ColorAtom, Conic, GradientAxis, Radial, StyleAtom, Token};
    let parsed = parse_class_string(
        "bg-radial bg-linear-to-r -bg-conic-45 from-accent via-40% to-surface bg-clip-text border-dashed border-dotted",
    )
    .expect("parse");
    let paint = StyleAtom::Paint;
    assert_eq!(
        parsed.as_slice(),
        [
            paint(PaintAtom::Gradient(GradientShape::Radial(Radial::new()))),
            paint(PaintAtom::Gradient(GradientShape::Linear(GradientAxis::LeftToRight))),
            paint(PaintAtom::Gradient(GradientShape::Conic(Conic::new(-45)))),
            paint(PaintAtom::Stop(StopSlot::From, ColorAtom::Token(Token::Accent))),
            paint(PaintAtom::StopAt(StopSlot::Via, 40)),
            paint(PaintAtom::Stop(StopSlot::To, ColorAtom::Token(Token::Surface))),
            paint(PaintAtom::ClipText),
            paint(PaintAtom::Border(BorderStyle::Dashed)),
            paint(PaintAtom::Border(BorderStyle::Dotted)),
        ]
    );
}
