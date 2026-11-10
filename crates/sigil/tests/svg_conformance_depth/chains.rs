use super::*;

fn linked(links: usize, link: impl Fn(usize, usize) -> String) -> String {
    (1..=links).map(|at| link(at, at - 1)).collect()
}

fn masked_tail(links: usize) -> String {
    format!(r##"<rect mask="url(#m{links})" width="24" height="24" fill="#fff"/></svg>"##)
}

const MASK_ZERO: &str = r##"<mask id="m0"><rect width="24" height="24" fill="#fff"/></mask>"##;

#[test]
fn a_chain_through_plain_groups_draws_nothing() {
    let links = linked(LINKS, |at, prior| {
        format!(
            r##"<g id="g{at}"><rect mask="url(#m{prior})" width="24" height="24"/></g><mask id="m{at}"><use href="#g{at}"/></mask>"##
        )
    });
    let tail = masked_tail(LINKS);
    assert!(!draws(format!(
        "{OPEN}<defs>{MASK_ZERO}{links}</defs>{tail}"
    )));
}

#[test]
fn a_chain_through_css_classes_draws_nothing() {
    let rules = linked(LINKS, |at, prior| format!(".k{at}{{mask:url(#m{prior})}}"));
    let links = linked(LINKS, |at, _| {
        format!(
            r##"<mask id="m{at}"><rect class="k{at}" width="24" height="24" fill="#fff"/></mask>"##
        )
    });
    let tail = masked_tail(LINKS);
    assert!(!draws(format!(
        "{OPEN}<style>{rules}</style>{MASK_ZERO}{links}{tail}"
    )));
}

#[test]
fn a_chain_through_css_id_selectors_draws_nothing() {
    let rules = linked(LINKS, |at, prior| format!("#r{at}{{mask:url(#m{prior})}}"));
    let links = linked(LINKS, |at, _| {
        format!(
            r##"<mask id="m{at}"><rect id="r{at}" width="24" height="24" fill="#fff"/></mask>"##
        )
    });
    let tail = masked_tail(LINKS);
    assert!(!draws(format!(
        "{OPEN}<style>{rules}</style>{MASK_ZERO}{links}{tail}"
    )));
}

#[test]
fn a_reference_held_in_a_custom_entity_still_counts() {
    let links = linked(LINKS, |at, prior| {
        format!(r#"<!ENTITY r{at} "url(#m{prior})">"#)
    });
    let masks = linked(LINKS, |at, _| {
        format!(
            r##"<mask id="m{at}"><rect mask="&r{at};" width="24" height="24" fill="#fff"/></mask>"##
        )
    });
    let tail = masked_tail(LINKS);
    assert!(!draws(format!(
        "<!DOCTYPE svg [{links}]>{OPEN}{MASK_ZERO}{masks}{tail}"
    )));
}

#[test]
fn entity_expansion_returns_in_bounded_time() {
    let mut entities = String::from(r#"<!ENTITY e0 "<g/><g/><g/><g/><g/><g/><g/><g/><g/><g/>">"#);
    for level in 1..=9 {
        let inner = format!("&e{};", level - 1).repeat(10);
        write!(entities, r#"<!ENTITY e{level} "{inner}">"#).expect("write");
    }
    let start = std::time::Instant::now();
    assert!(!draws(format!(
        "<!DOCTYPE svg [{entities}]>{OPEN}&e9;{INK}</svg>"
    )));
    assert!(start.elapsed().as_secs() < FAN_OUT_SECONDS);
}

#[test]
fn a_namespace_entity_on_the_root_still_draws() {
    let source = format!(
        r#"<!DOCTYPE svg [<!ENTITY ns "http://www.w3.org/2000/svg">]><svg xmlns="&ns;" viewBox="0 0 24 24">{INK}</svg>"#
    );
    assert!(draws(source));
}

#[test]
fn illustrator_classes_that_name_gradients_draw() {
    let rules = linked(200, |at, _| format!(".cls-{at}{{fill:url(#g)}}"));
    let shapes = linked(200, |at, _| {
        format!(r#"<rect class="cls-{at}" width="24" height="24"/>"#)
    });
    assert!(draws(format!(
        "{OPEN}<style>{rules}</style>{GRADIENT}{shapes}</svg>"
    )));
}

#[test]
fn a_reference_cycle_returns() {
    let source = format!(
        r##"{OPEN}<mask id="a"><rect mask="url(#b)" width="24" height="24" fill="#fff"/></mask><mask id="b"><rect mask="url(#a)" width="24" height="24" fill="#fff"/></mask><rect mask="url(#a)" width="24" height="24" fill="#fff"/>{INK}</svg>"##
    );
    let _returned = draws(source);
}

#[test]
fn a_long_reference_cycle_returns_and_draws_nothing() {
    let links = linked(LINKS, |at, prior| {
        format!(
            r##"<mask id="m{at}"><rect mask="url(#m{prior})" width="24" height="24" fill="#fff"/></mask>"##
        )
    });
    let closing = format!(
        r##"<mask id="m0"><rect mask="url(#m{LINKS})" width="24" height="24" fill="#fff"/></mask>"##
    );
    let tail = masked_tail(LINKS);
    assert!(!draws(format!("{OPEN}{closing}{links}{tail}")));
}

fn use_fan_out(levels: usize, width: usize) -> String {
    let mut defs = String::from(r##"<g id="u0"><rect width="24" height="24" fill="#fff"/></g>"##);
    for level in 1..=levels {
        let uses = format!(r##"<use href="#u{}"/>"##, level - 1).repeat(width);
        write!(defs, r#"<g id="u{level}">{uses}</g>"#).expect("write");
    }
    format!(r##"{OPEN}<defs>{defs}</defs><use href="#u{levels}"/></svg>"##)
}

fn pattern_fan_out(levels: usize, width: usize) -> String {
    let tile = r#"width="24" height="24" patternUnits="userSpaceOnUse""#;
    let mut defs = format!(
        r##"<pattern id="p0" {tile}><rect width="24" height="24" fill="#fff"/></pattern>"##
    );
    for level in 1..=levels {
        let prior = level - 1;
        let shapes =
            format!(r#"<rect width="24" height="24" fill="url(#p{prior})"/>"#).repeat(width);
        write!(defs, r#"<pattern id="p{level}" {tile}>{shapes}</pattern>"#).expect("write");
    }
    format!(r#"{OPEN}{defs}<rect width="24" height="24" fill="url(#p{levels})"/></svg>"#)
}

#[test]
fn a_use_fan_out_returns_in_bounded_time_and_draws_nothing() {
    let start = std::time::Instant::now();
    assert!(!draws(use_fan_out(10, 10)));
    assert!(start.elapsed().as_secs() < FAN_OUT_SECONDS);
}

#[test]
fn a_pattern_fan_out_returns_in_bounded_time_and_draws_nothing() {
    let start = std::time::Instant::now();
    assert!(!draws(pattern_fan_out(10, 10)));
    assert!(start.elapsed().as_secs() < FAN_OUT_SECONDS);
}

#[test]
fn a_small_use_fan_out_draws() {
    assert!(draws(use_fan_out(3, 10)));
}
