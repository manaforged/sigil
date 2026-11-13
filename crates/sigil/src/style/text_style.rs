/// Gives line height as a multiple of font size or in pixels.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextLineHeight {
    /// A multiple of the font size; the default is 1.2.
    FontSizeRelative(f32),
    /// A fixed line height in pixels.
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
    /// The font family name, such as `sans` or `mono`; the default is `system-ui`.
    pub font_family: String,
    /// The font size in pixels; the default is 14.
    pub font_size: f32,
    /// The font weight from 1 to 1000; the default is 400.
    pub font_weight: u16,
    /// Whether the text is italic.
    pub italic: bool,
    /// The line height, as a multiple of the font size or in pixels.
    pub line_height: TextLineHeight,
    /// The extra space between letters, in ems.
    pub letter_spacing_em: f32,
    /// Whether digits all take the same width.
    pub tabular_nums: bool,
    /// The line drawn with the text: none, underline, or line-through.
    pub decoration: crate::style::TextDecoration,
    /// The styled runs that override this style over byte ranges; `None` for plain text.
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
    /// Whether the run is bold: weight 700 unless the base weight is already 700 or more.
    pub bold: bool,
    /// Whether the run is italic.
    pub italic: bool,
    /// Whether the run uses the `mono` font family.
    pub mono: bool,
    /// Whether a line is drawn through the run.
    pub strike: bool,
    /// The theme token for the text color of the run, when it sets one.
    pub color: Option<crate::style::Token>,
    /// The theme token for the background behind the run, when it sets one.
    pub background: Option<crate::style::Token>,
    /// The exact color, if any; it wins over `color`.
    pub ink: Option<crate::style::Color>,
    /// The exact background, if any; it wins over `background`.
    pub fill: Option<crate::style::Color>,
    /// Whether a line is drawn under the run.
    pub underline: bool,
    /// Whether the run is drawn fainter than the text around it.
    pub dim: bool,
    /// Whether this is a terminal cell: a square background on a fixed character grid.
    pub square: bool,
    /// The link target that the run carries; `None` when the run is not a link.
    pub link: Option<std::sync::Arc<str>>,
}

/// A byte range of text with a face.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TextRun {
    /// The byte offset where the run starts, inclusive.
    pub start: usize,
    /// The byte offset where the run ends, exclusive.
    pub end: usize,
    /// The overrides that apply to this byte range.
    pub face: TextFace,
}

/// The styled runs of a text, sorted by byte offset.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextRuns(
    /// The runs in order of their start byte offset.
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
