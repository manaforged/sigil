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
#[non_exhaustive]
pub struct TextStyle {
    pub(crate) font_family: String,
    pub(crate) font_size: f32,
    pub(crate) font_weight: u16,
    pub(crate) italic: bool,
    pub(crate) line_height: TextLineHeight,
    pub(crate) letter_spacing_em: f32,
    pub(crate) tabular_nums: bool,
    pub(crate) decoration: crate::style::TextDecoration,
    pub(crate) runs: Option<TextRuns>,
    pub(crate) font_settings: crate::text::FontSettings,
}

/// The overrides of one styled run of text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent text attribute that combines freely with the others"
)]
#[non_exhaustive]
pub struct TextFace {
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) mono: bool,
    pub(crate) strike: bool,
    pub(crate) color: Option<crate::style::Token>,
    pub(crate) background: Option<crate::style::Token>,
    pub(crate) ink: Option<crate::style::Color>,
    pub(crate) fill: Option<crate::style::Color>,
    pub(crate) underline: bool,
    pub(crate) dim: bool,
    pub(crate) square: bool,
    pub(crate) link: Option<std::sync::Arc<str>>,
    pub(crate) font_settings: crate::text::FontSettings,
    pub(crate) weight: Option<u16>,
    pub(crate) family: Option<std::sync::Arc<str>>,
}

impl TextFace {
    /// Creates a face with no overrides set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bold: false,
            italic: false,
            mono: false,
            strike: false,
            color: None,
            background: None,
            ink: None,
            fill: None,
            underline: false,
            dim: false,
            square: false,
            link: None,
            font_settings: crate::text::FontSettings::new(),
            weight: None,
            family: None,
        }
    }

    /// Sets whether the run is bold.
    #[must_use]
    pub const fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Sets whether the run is italic.
    #[must_use]
    pub const fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Sets a variable-font axis of the run, such as `wdth` or `slnt`, in the axis's own units.
    #[must_use]
    pub fn axis(mut self, tag: &str, value: f32) -> Self {
        self.font_settings.axis(tag, value);
        self
    }

    /// Sets an OpenType feature of the run, such as `ss01` or `liga`, where 0 turns it off.
    #[must_use]
    pub fn feature(mut self, tag: &str, value: u16) -> Self {
        self.font_settings.feature(tag, value);
        self
    }

    /// Sets the font weight of the run from 1 to 1000.
    #[must_use]
    pub fn weight(mut self, weight: impl Into<Option<u16>>) -> Self {
        self.weight = weight.into();
        self
    }

    /// Sets the font family of the run.
    #[must_use]
    pub fn family(mut self, family: impl Into<Option<std::sync::Arc<str>>>) -> Self {
        self.family = family.into();
        self
    }

    /// Sets whether the run uses the `mono` font family.
    #[must_use]
    pub const fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Sets whether a line is drawn through the run.
    #[must_use]
    pub const fn strike(mut self, strike: bool) -> Self {
        self.strike = strike;
        self
    }

    /// Sets whether a line is drawn under the run.
    #[must_use]
    pub const fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// Sets whether the run is drawn fainter than surrounding text.
    #[must_use]
    pub const fn dim(mut self, dim: bool) -> Self {
        self.dim = dim;
        self
    }

    /// Sets whether this is a terminal cell background on a fixed character grid.
    #[must_use]
    pub const fn square(mut self, square: bool) -> Self {
        self.square = square;
        self
    }

    /// Sets the theme token for the text color of the run.
    #[must_use]
    pub fn color(mut self, color: impl Into<Option<crate::style::Token>>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the theme token for the background behind the run.
    #[must_use]
    pub fn background(mut self, background: impl Into<Option<crate::style::Token>>) -> Self {
        self.background = background.into();
        self
    }

    /// Sets the exact text color of the run.
    #[must_use]
    pub fn ink(mut self, ink: impl Into<Option<crate::style::Color>>) -> Self {
        self.ink = ink.into();
        self
    }

    /// Sets the exact background color behind the run.
    #[must_use]
    pub fn fill(mut self, fill: impl Into<Option<crate::style::Color>>) -> Self {
        self.fill = fill.into();
        self
    }

    /// Sets the link target carried by the run.
    #[must_use]
    pub fn link(mut self, link: impl Into<Option<std::sync::Arc<str>>>) -> Self {
        self.link = link.into();
        self
    }
}

/// A byte range of text with a face.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TextRun {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) face: TextFace,
}

impl TextRun {
    /// Creates a run that applies `face` to the bytes from `start` up to `end`.
    #[must_use]
    pub const fn new(start: usize, end: usize, face: TextFace) -> Self {
        Self { start, end, face }
    }

    /// The byte offset where the run starts, inclusive.
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }

    /// The byte offset where the run ends, exclusive.
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// The overrides that apply to this byte range.
    #[must_use]
    pub const fn face(&self) -> &TextFace {
        &self.face
    }
}

/// The styled runs of a text, sorted by byte offset.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextRuns(pub(crate) std::sync::Arc<[TextRun]>);

impl TextRuns {
    /// Creates styled runs from a slice, vector, or arc of runs.
    #[must_use]
    pub fn new(runs: impl Into<std::sync::Arc<[TextRun]>>) -> Self {
        Self(runs.into())
    }

    /// Returns the runs in order of their start byte offset.
    #[must_use]
    pub fn as_slice(&self) -> &[TextRun] {
        &self.0
    }

    /// Returns the number of runs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether there are no runs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the runs.
    pub fn iter(&self) -> core::slice::Iter<'_, TextRun> {
        self.0.iter()
    }

    /// Returns the run that holds `byte`.
    #[must_use]
    pub fn at(&self, byte: usize) -> Option<&TextRun> {
        self.0
            .iter()
            .find(|run| run.start <= byte && byte < run.end)
    }

    /// Returns the family of `face`, or `base` when it sets none.
    #[must_use]
    pub fn family<'a>(face: &'a TextFace, base: &'a str) -> &'a str {
        match &face.family {
            Some(family) => family,
            None if face.mono => "mono",
            None => base,
        }
    }

    /// Returns the weight of `face`, or `base` when it sets none.
    #[must_use]
    pub const fn weight(face: &TextFace, base: u16) -> u16 {
        match face.weight {
            Some(weight) if weight < 1 => 1,
            Some(weight) if weight > 1000 => 1000,
            Some(weight) => weight,
            None if face.bold && base < 700 => 700,
            None => base,
        }
    }
}

impl From<Vec<TextRun>> for TextRuns {
    fn from(runs: Vec<TextRun>) -> Self {
        Self(runs.into())
    }
}

impl<'a> IntoIterator for &'a TextRuns {
    type Item = &'a TextRun;
    type IntoIter = core::slice::Iter<'a, TextRun>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl TextStyle {
    /// Sets the font family name; the default is `system-ui`.
    #[must_use]
    pub fn font_family(mut self, font_family: impl Into<String>) -> Self {
        self.font_family = font_family.into();
        self
    }

    /// Sets the font size in pixels; the default is 14.
    #[must_use]
    pub const fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Sets the font weight from 100 to 900; the default is 400.
    #[must_use]
    pub const fn font_weight(mut self, font_weight: u16) -> Self {
        self.font_weight = font_weight;
        self
    }

    /// Sets whether the text is italic.
    #[must_use]
    pub const fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Sets the line height.
    #[must_use]
    pub const fn line_height(mut self, line_height: TextLineHeight) -> Self {
        self.line_height = line_height;
        self
    }

    /// Sets the extra space between letters, as a fraction of the font size.
    #[must_use]
    pub const fn letter_spacing_em(mut self, letter_spacing_em: f32) -> Self {
        self.letter_spacing_em = letter_spacing_em;
        self
    }

    /// Sets whether digits all take the same width.
    #[must_use]
    pub const fn tabular_nums(mut self, tabular_nums: bool) -> Self {
        self.tabular_nums = tabular_nums;
        self
    }

    /// Sets a variable-font axis such as `wdth`, `opsz`, `slnt`, or `wght` in the axis's own units.
    #[must_use]
    pub fn axis(mut self, tag: &str, value: f32) -> Self {
        self.font_settings.axis(tag, value);
        self
    }

    /// Sets an OpenType feature such as `ss01`, `calt`, or `liga`, where 0 turns it off.
    #[must_use]
    pub fn feature(mut self, tag: &str, value: u16) -> Self {
        self.font_settings.feature(tag, value);
        self
    }

    /// Sets the underline or strike line.
    #[must_use]
    pub const fn decoration(mut self, decoration: crate::style::TextDecoration) -> Self {
        self.decoration = decoration;
        self
    }

    /// Sets the styled runs that override this style over byte ranges.
    #[must_use]
    pub fn runs(mut self, runs: TextRuns) -> Self {
        self.runs = Some(runs);
        self
    }

    /// The font size in pixels.
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    #[must_use]
    pub const fn size_px(&self) -> f32 {
        self.font_size
    }

    /// The font weight from 1 to 1000.
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    #[must_use]
    pub const fn weight(&self) -> u16 {
        self.font_weight
    }

    /// The line height.
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    #[must_use]
    pub const fn height(&self) -> TextLineHeight {
        self.line_height
    }

    /// The extra space between letters, in ems.
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    #[must_use]
    pub const fn spacing_em(&self) -> f32 {
        self.letter_spacing_em
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: String::from("system-ui"),
            font_size: 14.0,
            font_weight: 400,
            font_settings: crate::text::FontSettings::new(),
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
