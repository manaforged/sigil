#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::cast_precision_loss,
    reason = "pixel sizes and step counts are small"
)]
#![cfg(feature = "kittest")]

use std::fmt::Write as _;

use sigil::prelude::*;
use sigil::widgets::icon_svg;
use sigil_kittest::Harness;

const SIDE: u32 = 32;
const STACK: usize = 2 << 20;
const LINKS: usize = 20_000;
const LIMIT: usize = 32;
const FAN_OUT_SECONDS: u64 = 30;
const BLACK: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
const INK: &str = r##"<rect width="24" height="24" fill="#fff"/>"##;
const OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">"#;

fn nested(depth: usize) -> String {
    format!(
        "{OPEN}{}{INK}{}</svg>",
        "<g>".repeat(depth),
        "</g>".repeat(depth)
    )
}

fn draws(svg: String) -> bool {
    let frame = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || {
            let app = move || {
                column()
                    .background(BLACK)
                    .child(icon_svg(svg.clone()).px(SIDE as f32))
            };
            let mut ui = Harness::new(app).pixels(SIDE, SIDE);
            ui.image().expect("harness image").expect("gpu frame")
        })
        .expect("thread")
        .join()
        .expect("no panic");
    frame.pixels().any(|pixel| pixel.0[0] > 200)
}

#[test]
fn an_icon_with_200000_nested_groups_returns_and_draws_nothing() {
    assert!(!draws(nested(200_000)));
}

#[test]
fn an_icon_nested_just_past_the_limit_draws_nothing() {
    assert!(!draws(nested(128)));
}

#[test]
fn an_icon_nested_100_groups_deep_draws() {
    assert!(draws(nested(100)));
}

#[test]
fn an_icon_at_the_nesting_limit_draws() {
    assert!(draws(nested(127)));
}

#[test]
fn nesting_inside_a_comment_or_attribute_does_not_count() {
    let comment = format!("<!--{}-->", "<g>".repeat(200));
    let attribute = format!(r#"<title id="{}"/>"#, "&lt;g&gt;".repeat(200));
    assert!(draws(format!("{OPEN}{comment}{attribute}{INK}</svg>")));
}

#[test]
fn an_icon_source_past_the_size_limit_draws_nothing() {
    let padding = format!("<!--{}-->", "x".repeat(33 << 20));
    assert!(!draws(format!("{OPEN}{padding}{INK}</svg>")));
}

#[test]
fn groups_kept_by_usvg_at_the_nesting_limit_lower_without_overflow() {
    let groups = r#"<g style="isolation:isolate">"#.repeat(127);
    let source = format!("{OPEN}{groups}{INK}{}</svg>", "</g>".repeat(127));
    assert!(draws(source));
}

fn clip_chain(links: usize) -> String {
    let mut defs = String::from(r#"<clipPath id="c0"><rect width="24" height="24"/></clipPath>"#);
    for link in 1..=links {
        write!(
            defs,
            r#"<clipPath id="c{link}" clip-path="url(#c{})"><rect width="24" height="24"/></clipPath>"#,
            link - 1
        )
        .expect("write");
    }
    format!(
        r##"{OPEN}{defs}<rect clip-path="url(#c{links})" width="24" height="24" fill="#fff"/></svg>"##
    )
}

fn mask_chain(links: usize) -> String {
    let mut defs =
        String::from(r##"<mask id="m0"><rect width="24" height="24" fill="#fff"/></mask>"##);
    for link in 1..=links {
        let prior = link - 1;
        write!(
            defs,
            r##"<mask id="m{link}"><rect mask="url(#m{prior})" width="24" height="24" fill="#fff"/></mask>"##
        )
        .expect("write");
    }
    format!(
        r##"{OPEN}{defs}<rect mask="url(#m{links})" width="24" height="24" fill="#fff"/></svg>"##
    )
}

fn pattern_chain(links: usize) -> String {
    let tile = r#"width="24" height="24" patternUnits="userSpaceOnUse""#;
    let mut defs = format!(
        r##"<pattern id="p0" {tile}><rect width="24" height="24" fill="#fff"/></pattern>"##
    );
    for link in 1..=links {
        let prior = link - 1;
        write!(
            defs,
            r#"<pattern id="p{link}" {tile}><rect width="24" height="24" fill="url(#p{prior})"/></pattern>"#
        )
        .expect("write");
    }
    format!(r#"{OPEN}{defs}<rect width="24" height="24" fill="url(#p{links})"/></svg>"#)
}

#[test]
fn long_reference_chains_return_and_draw_nothing() {
    for chain in [clip_chain, mask_chain, pattern_chain] {
        assert!(!draws(chain(LINKS)));
    }
}

#[test]
fn a_short_reference_chain_draws() {
    assert!(draws(mask_chain(16)));
}

#[test]
fn a_shallow_mask_chain_past_the_link_limit_draws_nothing() {
    assert!(!draws(mask_chain(LIMIT * 2)));
}

#[test]
fn reference_chains_at_the_limit_lower_without_overflow() {
    for chain in [clip_chain, mask_chain, pattern_chain] {
        let _returned = draws(chain(LIMIT));
    }
}

#[test]
fn a_reference_written_with_a_character_entity_still_counts() {
    let mut defs = String::new();
    for link in 1..=LINKS {
        let prior = link - 1;
        write!(
            defs,
            r#"<clipPath id="c{link}" clip-path="&#117;rl(&#35;c{prior})"><rect width="24" height="24"/></clipPath>"#
        )
        .expect("write");
    }
    assert!(!draws(format!(
        r##"{OPEN}<clipPath id="c0"><rect width="24" height="24"/></clipPath>{defs}<rect clip-path="url(#c{LINKS})" width="24" height="24" fill="#fff"/></svg>"##
    )));
}

#[test]
fn groups_expanded_from_an_entity_return_and_draw_nothing() {
    let depth = 100_000;
    let value = format!("{}{INK}{}", "<g>".repeat(depth), "</g>".repeat(depth));
    let source = format!(r"<!DOCTYPE svg [<!ENTITY deep '{value}'>]>{OPEN}&deep;</svg>");
    assert!(!draws(source));
}

#[test]
fn a_plain_text_entity_still_draws() {
    let source =
        format!(r#"<!DOCTYPE svg [<!ENTITY ns "http://www.w3.org/2000/svg">]>{OPEN}{INK}</svg>"#);
    assert!(draws(source));
}

fn deep_mask_chain(links: usize, depth: usize) -> String {
    let open = r#"<g style="isolation:isolate">"#.repeat(depth);
    let close = "</g>".repeat(depth);
    let mut defs = format!(
        r##"<mask id="m0">{open}<rect width="24" height="24" fill="#fff"/>{close}</mask>"##
    );
    for link in 1..=links {
        let prior = link - 1;
        write!(
            defs,
            r##"<mask id="m{link}">{open}<rect mask="url(#m{prior})" width="24" height="24" fill="#fff"/>{close}</mask>"##
        )
        .expect("write");
    }
    format!(
        r##"{OPEN}{defs}<rect mask="url(#m{links})" width="24" height="24" fill="#fff"/></svg>"##
    )
}

#[test]
fn groups_in_each_link_of_a_reference_chain_stay_bounded() {
    for (links, depth) in [(31, 6), (15, 14), (7, 30), (3, 62), (1, 126)] {
        let _returned = draws(deep_mask_chain(links, depth));
    }
    assert!(!draws(deep_mask_chain(31, 100)));
}

#[test]
fn hex_colors_in_a_definition_are_not_references() {
    let stops = r##"<rect width="24" height="24" fill="#fff"/>"##.repeat(100);
    let source = format!(
        r##"{OPEN}<mask id="m">{stops}</mask><rect mask="url(#m)" width="24" height="24" fill="#fff"/></svg>"##
    );
    assert!(draws(source));
}

const GRADIENT: &str = r##"<linearGradient id="g"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#fff"/></linearGradient>"##;

#[test]
fn sibling_clip_paths_that_use_one_shape_draw() {
    let shape = r#"<rect id="a" width="24" height="24"/>"#;
    let mut defs = format!("<defs>{shape}");
    let mut body = String::new();
    for index in 0..200 {
        write!(
            defs,
            r##"<clipPath id="b{index}"><use href="#a"/></clipPath>"##
        )
        .expect("write");
        write!(
            body,
            r##"<rect clip-path="url(#b{index})" width="24" height="24" fill="#fff"/>"##
        )
        .expect("write");
    }
    assert!(draws(format!("{OPEN}{defs}</defs>{body}</svg>")));
}

#[test]
fn a_mask_with_many_gradient_filled_shapes_draws() {
    let shapes = r#"<rect width="24" height="24" fill="url(#g)"/>"#.repeat(200);
    let source = format!(
        r##"{OPEN}{GRADIENT}<mask id="m">{shapes}</mask><rect mask="url(#m)" width="24" height="24" fill="#fff"/></svg>"##
    );
    assert!(draws(source));
}

#[test]
fn deep_groups_with_many_references_draw() {
    let shapes = r#"<rect width="24" height="24" fill="url(#g)"/>"#.repeat(50);
    let source = format!(
        r##"{OPEN}{GRADIENT}<mask id="m">{}{shapes}{}</mask><rect mask="url(#m)" width="24" height="24" fill="#fff"/></svg>"##,
        "<g>".repeat(17),
        "</g>".repeat(17),
    );
    assert!(draws(source));
}

#[test]
fn escaped_ampersands_in_a_definition_are_not_references() {
    let shapes = r##"<rect data-note="a&amp;b" width="24" height="24" fill="#fff"/>"##.repeat(200);
    let source = format!(
        r##"{OPEN}<mask id="m">{shapes}</mask><rect mask="url(#m)" width="24" height="24" fill="#fff"/></svg>"##
    );
    assert!(draws(source));
}

#[path = "svg_conformance_depth/chains.rs"]
mod chains;
