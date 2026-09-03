use sigil_hal::TextEngine;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextLineHeight {
    FontSizeRelative(f32),
    Absolute(f32),
}

impl Default for TextLineHeight {
    fn default() -> Self {
        Self::FontSizeRelative(1.2)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub italic: bool,
    pub line_height: TextLineHeight,
    pub letter_spacing_em: f32,
    pub tabular_nums: bool,
    pub decoration: crate::TextDecoration,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: String::from("system-ui"),
            font_size: 14.0,
            font_weight: 400,
            italic: false,
            line_height: TextLineHeight::default(),
            letter_spacing_em: 0.0,
            tabular_nums: false,
            decoration: crate::TextDecoration::None,
        }
    }
}

#[must_use]
pub const fn is_typography_atom(atom: crate::StyleAtom) -> bool {
    matches!(
        atom,
        crate::StyleAtom::FontSize(_)
            | crate::StyleAtom::FontWeight(_)
            | crate::StyleAtom::FontItalic(_)
            | crate::StyleAtom::FontFamily(_)
            | crate::StyleAtom::LineHeight(_)
            | crate::StyleAtom::LetterSpacing(_)
            | crate::StyleAtom::TabularNums(_)
            | crate::StyleAtom::TextDecoration(_)
    )
}

impl TextStyle {
    #[must_use]
    pub fn layered_over_binding(base: &Self, binding: &crate::StyleBinding) -> Option<Self> {
        let base_has = [binding.static_slice(), binding.dynamic_slice()]
            .iter()
            .any(|slice| slice.iter().any(|atom| is_typography_atom(*atom)));
        let reactive_has = binding
            .reactive_channels()
            .any(|slice| slice.iter().any(|atom| is_typography_atom(*atom)));
        if !(base_has || reactive_has) {
            return None;
        }
        let mut layered = base.clone();
        crate::engine::apply_text_atoms(binding.static_slice(), &mut layered);
        crate::engine::apply_text_atoms(binding.dynamic_slice(), &mut layered);
        for reactive in binding.reactive_channels() {
            crate::engine::apply_text_atoms(reactive, &mut layered);
        }
        Some(layered)
    }
}

pub trait StyledTextEngine: TextEngine {
    fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Self::Paragraph, Self::Error>;
}
