/// Gives line height as a multiple of font size or in pixels.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextLineHeight {
    /// The font size relative text line height.
    FontSizeRelative(f32),
    /// The absolute text line height.
    Absolute(f32),
}

impl Default for TextLineHeight {
    fn default() -> Self {
        Self::FontSizeRelative(1.2)
    }
}

/// Holds the resolved typography of a text node.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    /// The font family.
    pub font_family: String,
    /// The font size.
    pub font_size: f32,
    /// The font weight.
    pub font_weight: u16,
    /// Whether italic.
    pub italic: bool,
    /// The line height.
    pub line_height: TextLineHeight,
    /// The letter spacing em.
    pub letter_spacing_em: f32,
    /// Whether tabular nums.
    pub tabular_nums: bool,
    /// The decoration.
    pub decoration: crate::style::TextDecoration,
    /// The runs, if any.
    pub runs: Option<TextRuns>,
}

/// The overrides of one styled run of text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent text attribute that combines freely with the others"
)]
#[non_exhaustive]
pub struct TextFace {
    /// Whether bold.
    pub bold: bool,
    /// Whether italic.
    pub italic: bool,
    /// Whether mono.
    pub mono: bool,
    /// Whether struck through.
    pub strike: bool,
    /// The color, if any.
    pub color: Option<crate::style::Token>,
    /// The background, if any.
    pub background: Option<crate::style::Token>,
    /// The exact color, if any; it wins over `color`.
    pub ink: Option<crate::style::Color>,
    /// The exact background, if any; it wins over `background`.
    pub fill: Option<crate::style::Color>,
    /// Whether underlined.
    pub underline: bool,
    /// Whether drawn fainter.
    pub dim: bool,
    /// Whether this is a terminal cell: a square background on a fixed character grid.
    pub square: bool,
    /// The link, if any.
    pub link: Option<std::sync::Arc<str>>,
}

/// A byte range of text with a face.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TextRun {
    /// The start.
    pub start: usize,
    /// The end.
    pub end: usize,
    /// The face.
    pub face: TextFace,
}

/// The styled runs of a text, sorted by byte offset.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextRuns(
    /// The runs.
    pub std::sync::Arc<[TextRun]>,
);

impl TextRuns {
    /// Returns the run that holds `byte`.
    #[must_use]
    pub fn at(&self, byte: usize) -> Option<&TextRun> {
        self.0
            .iter()
            .find(|run| run.start <= byte && byte < run.end)
    }

    /// Returns the family of `face`, or `base` when it sets none.
    #[must_use]
    pub const fn family<'a>(face: &TextFace, base: &'a str) -> &'a str {
        if face.mono { "mono" } else { base }
    }

    /// Returns the weight of `face`, or `base` when it sets none.
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

/// Returns true when a style atom changes text layout.
#[must_use]
pub(crate) const fn is_typography_atom(atom: crate::style::StyleAtom) -> bool {
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
    pub(crate) fn binding_layers_text(binding: &crate::style::StyleBinding) -> bool {
        binding.cascade().any(is_typography_atom)
            || binding.animator_overlay().is_some_and(|overlay| {
                overlay
                    .value_for(crate::style::NumericAtomKind::FontSize)
                    .is_some()
                    || overlay
                        .value_for(crate::style::NumericAtomKind::LineHeight)
                        .is_some()
            })
    }

    pub(crate) fn layered_over_binding(
        base: &Self,
        binding: &crate::style::StyleBinding,
    ) -> Option<Self> {
        if !Self::binding_layers_text(binding) {
            return None;
        }
        let mut layered = base.clone();
        for atom in binding.cascade() {
            crate::style::engine::apply_text_atoms(&[atom], &mut layered);
        }
        if let Some(overlay) = binding.animator_overlay() {
            crate::style::StyleEngine::overlay_text(overlay, &mut layered);
        }
        Some(layered)
    }
}
