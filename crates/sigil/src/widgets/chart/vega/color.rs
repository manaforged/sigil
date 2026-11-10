use crate::style::Color;

const NAMED: [(&str, u32); 24] = [
    ("black", 0x00_00_00),
    ("silver", 0xc0_c0_c0),
    ("gray", 0x80_80_80),
    ("grey", 0x80_80_80),
    ("white", 0xff_ff_ff),
    ("maroon", 0x80_00_00),
    ("red", 0xff_00_00),
    ("purple", 0x80_00_80),
    ("fuchsia", 0xff_00_ff),
    ("magenta", 0xff_00_ff),
    ("green", 0x00_80_00),
    ("lime", 0x00_ff_00),
    ("olive", 0x80_80_00),
    ("yellow", 0xff_ff_00),
    ("navy", 0x00_00_80),
    ("blue", 0x00_00_ff),
    ("teal", 0x00_80_80),
    ("aqua", 0x00_ff_ff),
    ("cyan", 0x00_ff_ff),
    ("orange", 0xff_a5_00),
    ("brown", 0xa5_2a_2a),
    ("pink", 0xff_c0_cb),
    ("gold", 0xff_d7_00),
    ("steelblue", 0x46_82_b4),
];

pub(super) fn css(name: &str) -> Option<Color> {
    let name = name.trim();
    if name.starts_with('#') {
        return name.parse().ok();
    }
    let lower = name.to_ascii_lowercase();
    let (_, rgb) = NAMED.iter().find(|(known, _)| *known == lower)?;
    let [_, r, g, b] = rgb.to_be_bytes();
    Some(Color { r, g, b, a: 255 })
}
