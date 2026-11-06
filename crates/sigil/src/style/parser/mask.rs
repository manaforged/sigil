use smallvec::SmallVec;

use crate::style::atom::{MaskAtom, MaskKind, MaskPart, StopSlot};
use crate::style::{Conic, MaskSize, RadialExtent, StyleAtom};

type Atoms = SmallVec<[StyleAtom; 16]>;

const PLACES: [(&str, [u8; 2]); 9] = [
    ("top-left", [0, 0]),
    ("top", [50, 0]),
    ("top-right", [100, 0]),
    ("left", [0, 50]),
    ("center", [50, 50]),
    ("right", [100, 50]),
    ("bottom-left", [0, 100]),
    ("bottom", [50, 100]),
    ("bottom-right", [100, 100]),
];

const EDGES: [(&str, i16); 4] = [("t", 0), ("r", 90), ("b", 180), ("l", 270)];

const KINDS: [(&str, MaskKind); 3] = [
    ("linear", MaskKind::Linear),
    ("radial", MaskKind::Radial),
    ("conic", MaskKind::Conic),
];

pub(super) fn push_mask_atoms(token: &str, atoms: &mut Atoms) -> bool {
    let (sign, rest) = token
        .strip_prefix('-')
        .map_or((1, token), |rest| (-1, rest));
    let Some(rest) = rest.strip_prefix("mask-") else {
        return false;
    };
    let parts = if sign < 0 {
        angle(rest, -1).map(|part| [Some(part), None])
    } else {
        keyword(rest)
            .or_else(|| place(rest))
            .map(|part| [Some(part), None])
            .or_else(|| stops(rest))
            .or_else(|| angle(rest, 1).map(|part| [Some(part), None]))
    };
    let Some(parts) = parts else {
        return false;
    };
    atoms.extend(
        parts
            .into_iter()
            .flatten()
            .map(|part| StyleAtom::Mask(MaskAtom(part))),
    );
    true
}

fn keyword(rest: &str) -> Option<MaskPart> {
    Some(match rest {
        "none" => MaskPart::None,
        "alpha" => MaskPart::Luminance(false),
        "luminance" => MaskPart::Luminance(true),
        "auto" => MaskPart::Size(MaskSize::Auto),
        "cover" => MaskPart::Size(MaskSize::Cover),
        "contain" => MaskPart::Size(MaskSize::Contain),
        "repeat" => MaskPart::Repeat(true),
        "no-repeat" => MaskPart::Repeat(false),
        "circle" => MaskPart::Circle(true),
        "ellipse" => MaskPart::Circle(false),
        "radial-closest-side" => MaskPart::Extent(RadialExtent::ClosestSide),
        "radial-closest-corner" => MaskPart::Extent(RadialExtent::ClosestCorner),
        "radial-farthest-side" => MaskPart::Extent(RadialExtent::FarthestSide),
        "radial-farthest-corner" => MaskPart::Extent(RadialExtent::FarthestCorner),
        _ => return None,
    })
}

fn place(rest: &str) -> Option<MaskPart> {
    let find = |name: &str| {
        PLACES
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, at)| *at)
    };
    if let Some(name) = rest.strip_prefix("radial-at-") {
        return find(name).map(MaskPart::RadialAt);
    }
    find(rest).map(|at| MaskPart::Position(at.map(|percent| u16::from(percent) * 100)))
}

fn angle(rest: &str, sign: i16) -> Option<MaskPart> {
    if let Some(degrees) = rest.strip_prefix("linear-") {
        return degrees
            .parse::<i16>()
            .ok()
            .map(|n| MaskPart::Linear(sign * (n % 360)));
    }
    let degrees = rest.strip_prefix("conic-")?.parse::<i16>().ok()?;
    Some(MaskPart::Conic(Conic::new(sign * (degrees % 360))))
}

fn stops(rest: &str) -> Option<[Option<MaskPart>; 2]> {
    let (head, slot, raw) = split_stop(rest)?;
    let percent = raw
        .strip_suffix('%')?
        .parse::<u8>()
        .ok()
        .filter(|n| *n <= 100)?;
    if let Some((_, kind)) = KINDS.iter().find(|(name, _)| *name == head) {
        return Some([Some(MaskPart::Stop(Some(*kind), slot, percent)), None]);
    }
    let (_, degrees) = EDGES.iter().find(|(name, _)| *name == head)?;
    let stop = MaskPart::Stop(Some(MaskKind::Linear), slot, percent);
    Some([Some(MaskPart::Linear(*degrees)), Some(stop)])
}

fn split_stop(rest: &str) -> Option<(&str, StopSlot, &str)> {
    if let Some((head, raw)) = rest.split_once("-from-") {
        return Some((head, StopSlot::From, raw));
    }
    let (head, raw) = rest.split_once("-to-")?;
    Some((head, StopSlot::To, raw))
}
