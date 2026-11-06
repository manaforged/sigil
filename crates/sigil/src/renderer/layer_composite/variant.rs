use crate::scene::command::BlendMode;

pub(super) struct Variant {
    pub(super) label: &'static str,
    pub(super) groups: usize,
    pub(super) fs_entry: &'static str,
    pub(super) blend: wgpu::BlendState,
}

pub(super) const MASKED: Variant = Variant {
    label: "sigil-renderer masked layer composite",
    groups: 3,
    fs_entry: "fs_masked",
    blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
};

pub(super) const BLENDED: Variant = Variant {
    label: "sigil-renderer blended layer composite",
    groups: 4,
    fs_entry: "fs_blend",
    blend: wgpu::BlendState::REPLACE,
};

pub(super) const fn blend_index(mode: BlendMode) -> u32 {
    match mode {
        BlendMode::Normal | BlendMode::SrgbOver => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Overlay => 3,
        BlendMode::Darken => 4,
        BlendMode::Lighten => 5,
        BlendMode::ColorDodge => 6,
        BlendMode::ColorBurn => 7,
        BlendMode::HardLight => 8,
        BlendMode::SoftLight => 9,
        BlendMode::Difference => 10,
        BlendMode::Exclusion => 11,
        BlendMode::Hue => 12,
        BlendMode::Saturation => 13,
        BlendMode::Color => 14,
        BlendMode::Luminosity => 15,
    }
}
