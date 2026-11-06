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
            paint(PaintAtom::Gradient(GradientShape::Linear(
                GradientAxis::LeftToRight
            ))),
            paint(PaintAtom::Gradient(GradientShape::Conic(Conic::new(-45)))),
            paint(PaintAtom::Stop(
                StopSlot::From,
                ColorAtom::Token(Token::Accent)
            )),
            paint(PaintAtom::StopAt(StopSlot::Via, 40)),
            paint(PaintAtom::Stop(
                StopSlot::To,
                ColorAtom::Token(Token::Surface)
            )),
            paint(PaintAtom::ClipText),
            paint(PaintAtom::Border(BorderStyle::Dashed)),
            paint(PaintAtom::Border(BorderStyle::Dotted)),
        ]
    );
}

fn mask_of(atoms: &[crate::style::StyleAtom]) -> crate::style::atom::MaskStyle {
    let mut render = crate::style::RenderStyle::default();
    crate::style::StyleEngine::new(&crate::style::Theme::default()).apply(atoms, &mut render);
    render.mask
}

#[test]
fn tailwind_mask_classes_resolve_like_the_mask_setters() {
    use crate::style::atom::{MaskKind, MaskStyle};
    use crate::style::{MaskImage, MaskSize, Paint, Style};
    let classes =
        "mask-r-from-20% mask-r-to-80% mask-luminance mask-cover mask-no-repeat mask-bottom-right";
    let parsed = parse_class_string(classes).expect("parse");
    let set = Style::new()
        .mask_image(MaskImage::Linear(90))
        .mask_stops(20, 80)
        .mask_luminance()
        .mask_size(MaskSize::Cover)
        .mask_repeat(false)
        .mask_position(1.0, 1.0)
        .into_atoms();
    let expected = MaskStyle {
        kind: Some(MaskKind::Linear),
        angle: 90,
        stops: [20, 80],
        luminance: true,
        size: MaskSize::Cover,
        position: [10_000, 10_000],
        repeat: false,
        ..MaskStyle::NONE
    };
    assert_eq!(mask_of(&parsed), expected, "{classes}");
    assert_eq!(mask_of(&set), expected, "the setters");
}

#[test]
fn tailwind_radial_and_conic_mask_classes_parse() {
    use crate::style::atom::{MaskKind, MaskStyle};
    use crate::style::{Conic, Radial, RadialExtent};
    let radial = parse_class_string("mask-circle mask-radial-closest-side mask-radial-at-top-left mask-radial-from-10% mask-alpha")
        .expect("parse");
    let expected = MaskStyle {
        kind: Some(MaskKind::Radial),
        radial: Radial::new()
            .circle(true)
            .extent(RadialExtent::ClosestSide)
            .at(0, 0),
        stops: [10, 100],
        ..MaskStyle::NONE
    };
    assert_eq!(mask_of(&radial), expected);
    let conic = parse_class_string("-mask-conic-45 mask-conic-to-75%").expect("parse");
    let expected = MaskStyle {
        kind: Some(MaskKind::Conic),
        conic: Conic::new(-45),
        stops: [0, 75],
        ..MaskStyle::NONE
    };
    assert_eq!(mask_of(&conic), expected);
    assert_eq!(
        mask_of(&parse_class_string("mask-linear-45 mask-none").expect("parse")).kind,
        None
    );
    for bad in ["mask-r-from-120%", "mask-sideways", "mask-linear-from-x"] {
        assert!(
            parse_class_string(bad).is_err(),
            "{bad} is not a mask class"
        );
    }
}

#[test]
fn tailwind_mix_blend_classes_parse_and_the_last_one_wins() {
    use crate::scene::BlendMode;
    use crate::style::{RenderStyle, StyleAtom, StyleEngine, Theme};
    let parsed =
        parse_class_string("mix-blend-color-dodge mix-blend-soft-light mix-blend-luminosity")
            .expect("parse");
    assert_eq!(
        parsed.as_slice(),
        [
            StyleAtom::MixBlend(BlendMode::ColorDodge),
            StyleAtom::MixBlend(BlendMode::SoftLight),
            StyleAtom::MixBlend(BlendMode::Luminosity),
        ]
    );
    let mut render = RenderStyle::default();
    StyleEngine::new(&Theme::default()).apply(&parsed, &mut render);
    assert_eq!(render.blend, BlendMode::Luminosity);
    assert!(parse_class_string("mix-blend-plus-lighter").is_err());
}

fn filters_of(classes: &str) -> crate::style::atom::FilterStyle {
    let atoms = parse_class_string(classes).expect("parse");
    let mut render = crate::style::RenderStyle::default();
    crate::style::StyleEngine::new(&crate::style::Theme::default()).apply(&atoms, &mut render);
    render.filter
}

#[test]
fn tailwind_filter_classes_build_the_list_in_tailwind_order() {
    use crate::scene::command::Filter;
    let found =
        filters_of("drop-shadow-md sepia hue-rotate-90 blur-md brightness-50 -hue-rotate-30")
            .filters();
    let kinds: Vec<_> = found.iter().map(std::mem::discriminant).collect();
    let expected = [
        Filter::Blur(12.0),
        Filter::Brightness(0.5),
        Filter::HueRotate(-30.0),
        Filter::Sepia(1.0),
    ];
    assert_eq!(found[..4], expected);
    assert_eq!(kinds.len(), 5, "{found:?}");
    let Filter::DropShadow(shadow) = found[4] else {
        panic!("drop-shadow comes last: {found:?}");
    };
    assert_eq!((shadow.offset, shadow.blur), ([0.0, 3.0], 3.0));
    assert!(
        (shadow.color[3] - 0.12).abs() < 0.01,
        "drop-shadow-md alpha {:?}",
        shadow.color
    );
    assert!(filters_of("blur-md blur-none").filters().is_empty());
    for bad in ["blur-huge", "brightness-x", "grayscalex"] {
        assert!(parse_class_string(bad).is_err(), "{bad}");
    }
}

#[test]
fn tailwind_text_shadow_classes_expand_to_the_tailwind_lists() {
    let shadows = filters_of("text-shadow-md").text_shadows();
    let found: Vec<_> = shadows.iter().map(|s| (s.offset[1], s.blur)).collect();
    assert_eq!(
        found,
        [(1.0, 0.5), (1.0, 1.0), (2.0, 2.0)],
        "radius 1, 2, 4 px is a deviation of half"
    );
    assert!(shadows.iter().all(|s| (s.color[3] - 0.1).abs() < 0.01));
    let tinted = filters_of("text-shadow-xs text-shadow-accent").text_shadows();
    assert_eq!(tinted.len(), 1);
    assert!(
        tinted[0].color[..3].iter().any(|c| *c > 0.05),
        "text-shadow-<color> tints the shadow: {:?}",
        tinted[0].color
    );
    assert!(
        filters_of("text-shadow-lg text-shadow-none")
            .text_shadows()
            .is_empty()
    );
}
