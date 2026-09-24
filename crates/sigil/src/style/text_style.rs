use crate::hal::TextEngine;

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
    pub decoration: crate::style::TextDecoration,
    pub runs: Option<TextRuns>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextFace {
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
    pub color: Option<crate::style::Token>,
    pub background: Option<crate::style::Token>,
    pub link: Option<std::sync::Arc<str>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextRun {
    pub start: usize,
    pub end: usize,
    pub face: TextFace,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextRuns(pub std::sync::Arc<[TextRun]>);

impl TextRuns {
    #[must_use]
    pub fn at(&self, byte: usize) -> Option<&TextRun> {
        self.0
            .iter()
            .find(|run| run.start <= byte && byte < run.end)
    }

    #[must_use]
    pub const fn family<'a>(face: &TextFace, base: &'a str) -> &'a str {
        if face.mono { "mono" } else { base }
    }

    #[must_use]
    pub const fn weight(face: &TextFace, base: u16) -> u16 {
        if face.bold && base < 700 { 700 } else { base }
    }
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
            decoration: crate::style::TextDecoration::None,
            runs: None,
        }
    }
}

#[must_use]
pub const fn is_typography_atom(atom: crate::style::StyleAtom) -> bool {
    matches!(
        atom,
        crate::style::StyleAtom::FontSize(_)
            | crate::style::StyleAtom::FontWeight(_)
            | crate::style::StyleAtom::FontItalic(_)
            | crate::style::StyleAtom::FontFamily(_)
            | crate::style::StyleAtom::LineHeight(_)
            | crate::style::StyleAtom::LetterSpacing(_)
            | crate::style::StyleAtom::TabularNums(_)
            | crate::style::StyleAtom::TextDecoration(_)
    )
}

impl TextStyle {
    #[must_use]
    pub fn layered_over_binding(base: &Self, binding: &crate::style::StyleBinding) -> Option<Self> {
        let base_has = [binding.static_slice(), binding.dynamic_slice()]
            .iter()
            .any(|slice| slice.iter().any(|atom| is_typography_atom(*atom)));
        let reactive_has = binding
            .reactive_channels()
            .any(|slice| slice.iter().any(|atom| is_typography_atom(*atom)));
        if !(base_has
            || reactive_has
            || binding.animator_overlay().is_some_and(|overlay| {
                overlay
                    .value_for(crate::style::NumericAtomKind::FontSize)
                    .is_some()
                    || overlay
                        .value_for(crate::style::NumericAtomKind::LineHeight)
                        .is_some()
            })
            || binding
                .state_overlay_atoms()
                .iter()
                .any(|atom| is_typography_atom(*atom)))
        {
            return None;
        }
        let mut layered = base.clone();
        crate::style::engine::apply_text_atoms(binding.static_slice(), &mut layered);
        crate::style::engine::apply_text_atoms(binding.dynamic_slice(), &mut layered);
        for reactive in binding.reactive_channels() {
            crate::style::engine::apply_text_atoms(reactive, &mut layered);
        }
        crate::style::engine::apply_text_atoms(binding.state_overlay_atoms(), &mut layered);
        if let Some(overlay) = binding.animator_overlay() {
            crate::style::StyleEngine::overlay_text(overlay, &mut layered);
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
