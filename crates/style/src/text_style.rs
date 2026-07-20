//! Resolved typography used by text measurement and painting.

use sigil_hal::TextEngine;

/// Resolved line-height value for paragraph layout.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextLineHeight {
    /// Unitless multiple of the font size.
    FontSizeRelative(f32),
    /// Absolute logical-pixel line height.
    Absolute(f32),
}

impl Default for TextLineHeight {
    fn default() -> Self {
        Self::FontSizeRelative(1.2)
    }
}

/// Resolved typography used by text measurement and painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    /// Font family name, e.g. `"system-ui"`.
    pub font_family: String,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// CSS-style font weight.
    pub font_weight: u16,
    /// Paragraph line-height.
    pub line_height: TextLineHeight,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: String::from("system-ui"),
            font_size: 16.0,
            font_weight: 400,
            line_height: TextLineHeight::default(),
        }
    }
}

/// True when the atom changes resolved typography (DR-056 inherited set:
/// font family, size, weight, line height).
#[must_use]
pub const fn is_typography_atom(atom: crate::StyleAtom) -> bool {
    matches!(
        atom,
        crate::StyleAtom::FontSize(_)
            | crate::StyleAtom::FontWeight(_)
            | crate::StyleAtom::FontFamily(_)
            | crate::StyleAtom::LineHeight(_)
    )
}

impl TextStyle {
    /// Layer a node's typography atoms over an inherited base (DR-056).
    ///
    /// Applies the binding's static, dynamic, and reactive slices in
    /// precedence order. State-overlay atoms are interaction-local and
    /// do not transmit to descendants; consumers that honour them for
    /// the node itself apply them on top of the returned value.
    ///
    /// Returns `None` when the binding carries no typography atoms, so
    /// hot-path callers (most containers) keep the inherited value by
    /// reference and allocate nothing.
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

/// Text engines that can apply resolved Sigil typography.
pub trait StyledTextEngine: TextEngine {
    /// Lay out a paragraph with explicit resolved typography.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if paragraph layout fails.
    fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Self::Paragraph, Self::Error>;
}
