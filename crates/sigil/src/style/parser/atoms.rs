use smallvec::SmallVec;

use crate::style::StyleAtom;

pub(super) fn push_border_side_atoms(token: &str, atoms: &mut SmallVec<[StyleAtom; 16]>) -> bool {
    let Some(rest) = token.strip_prefix("border-") else {
        return false;
    };
    let Some((sides, width)) = border_side_spec(rest) else {
        return false;
    };
    for atom in sides {
        match atom {
            't' => atoms.push(StyleAtom::BorderTop(width)),
            'r' => atoms.push(StyleAtom::BorderRight(width)),
            'b' => atoms.push(StyleAtom::BorderBottom(width)),
            'l' => atoms.push(StyleAtom::BorderLeft(width)),
            _ => {}
        }
    }
    true
}

fn border_side_spec(rest: &str) -> Option<(&'static [char], i16)> {
    let (side, width) = match rest.split_once('-') {
        Some((side, raw)) => (side, raw.parse::<i16>().ok().filter(|n| *n >= 0)?),
        None => (rest, 1),
    };
    let sides: &[char] = match side {
        "t" => &['t'],
        "r" => &['r'],
        "b" => &['b'],
        "l" => &['l'],
        "x" => &['l', 'r'],
        "y" => &['t', 'b'],
        _ => return None,
    };
    Some((sides, width))
}
