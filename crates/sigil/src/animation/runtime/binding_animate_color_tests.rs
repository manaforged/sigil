use super::*;

#[test]
fn srgb_roundtrip_at_extremes() {
    for c in [0u8, 1, 64, 128, 200, 254, 255] {
        let back = linear_channel_to_srgb(crate::style::srgb::decode(c));
        assert!(back.abs_diff(c) <= 1, "{c} -> {back}");
    }
}
