use super::color_key;
use crate::style::atom::{PaintAtom, StopSlot};
use crate::style::{Conic, StyleAtom};

const STOPS: [(&str, StopSlot); 3] = [
    ("from-", StopSlot::From),
    ("via-", StopSlot::Via),
    ("to-", StopSlot::To),
];

pub(super) fn parse_paint_token(token: &str) -> Option<StyleAtom> {
    conic(token).or_else(|| stop(token)).map(StyleAtom::Paint)
}

fn conic(token: &str) -> Option<PaintAtom> {
    let (sign, rest) = token
        .strip_prefix('-')
        .map_or((1, token), |rest| (-1, rest));
    let degrees = rest.strip_prefix("bg-conic-")?.parse::<i16>().ok()?;
    let shape = crate::style::atom::GradientShape::Conic(Conic::new(sign * (degrees % 360)));
    Some(PaintAtom::Gradient(shape))
}

fn stop(token: &str) -> Option<PaintAtom> {
    let (raw, slot) = STOPS
        .iter()
        .find_map(|(prefix, slot)| token.strip_prefix(prefix).map(|raw| (raw, *slot)))?;
    if let Some(number) = raw.strip_suffix('%') {
        let percent = number.parse::<u8>().ok().filter(|n| *n <= 100)?;
        return Some(PaintAtom::StopAt(slot, percent));
    }
    color_key(raw).map(|color| PaintAtom::Stop(slot, color))
}
