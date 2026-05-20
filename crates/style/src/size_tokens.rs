//! Named logical-pixel size vocabulary shared by Sigil widgets.

use crate::{Dimension, Px};

/// Named size token.
///
/// Tokens intentionally store logical pixels, not theme colours. Runtime
/// theme token maps remain a separate concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeToken {
    /// Small control height.
    ControlSm,
    /// Medium control height.
    ControlMd,
    /// Large control height.
    ControlLg,
    /// Default large control width.
    ControlLgWidth,
    /// Tooltip bubble width.
    TooltipBubbleWidth,
    /// Tooltip bubble height.
    TooltipBubbleHeight,
    /// Tooltip bubble padding.
    TooltipBubblePadding,
    /// Tab width.
    TabWidth,
    /// Tab height.
    TabHeight,
    /// Menu panel width.
    MenuWidth,
    /// Checkbox indicator box.
    CheckboxBox,
    /// Checkbox checked mark.
    CheckboxMark,
    /// Long slider track axis.
    SliderTrackLong,
    /// Short slider track axis.
    SliderTrackShort,
    /// Slider thumb.
    SliderThumb,
    /// Extra-small padding.
    PaddingXs,
    /// Small padding.
    PaddingSm,
    /// Medium padding.
    PaddingMd,
    /// Large padding.
    PaddingLg,
    /// Extra-small gap.
    GapXs,
    /// Small gap.
    GapSm,
    /// Medium gap.
    GapMd,
    /// Small radius.
    RadiusSm,
    /// Medium radius.
    RadiusMd,
    /// Large radius.
    RadiusLg,
}

impl SizeToken {
    /// Return this token as logical pixels.
    #[must_use]
    pub const fn px(self) -> f32 {
        self.px_i16() as f32
    }

    /// Return this token as a style-layer pixel literal.
    #[must_use]
    pub const fn style_px(self) -> Px {
        Px(self.px_i16())
    }

    /// Return this token as a style-layer dimension.
    #[must_use]
    pub const fn style_dimension(self) -> Dimension {
        Dimension::Px(self.style_px())
    }

    /// Return this token as an integer logical-pixel value.
    #[must_use]
    pub const fn px_i16(self) -> i16 {
        match self {
            Self::ControlSm => 24,
            Self::ControlMd => 32,
            Self::ControlLg => 44,
            Self::ControlLgWidth => 148,
            Self::TooltipBubbleWidth => 160,
            Self::TooltipBubblePadding => 6,
            Self::TabWidth => 80,
            Self::TabHeight => 34,
            Self::MenuWidth => 200,
            Self::CheckboxBox => 20,
            Self::SliderTrackLong => 180,
            Self::SliderThumb => 22,
            Self::TooltipBubbleHeight | Self::SliderTrackShort => 28,
            Self::CheckboxMark | Self::PaddingSm | Self::GapSm | Self::RadiusLg => 8,
            Self::PaddingXs | Self::GapXs | Self::RadiusMd => 4,
            Self::PaddingMd | Self::GapMd => 12,
            Self::PaddingLg => 16,
            Self::RadiusSm => 2,
        }
    }
}

impl From<SizeToken> for f32 {
    fn from(token: SizeToken) -> Self {
        token.px()
    }
}

impl From<SizeToken> for Px {
    fn from(token: SizeToken) -> Self {
        token.style_px()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_tokens_match_widget_standards() {
        assert_eq!(SizeToken::ControlSm.px_i16(), 24);
        assert_eq!(SizeToken::ControlMd.px_i16(), 32);
        assert_eq!(SizeToken::ControlLg.px_i16(), 44);
    }

    #[test]
    fn spacing_tokens_match_named_steps() {
        assert_eq!(SizeToken::PaddingXs.px_i16(), 4);
        assert_eq!(SizeToken::PaddingSm.px_i16(), 8);
        assert_eq!(SizeToken::PaddingMd.px_i16(), 12);
        assert_eq!(SizeToken::PaddingLg.px_i16(), 16);
        assert_eq!(SizeToken::GapXs.px_i16(), 4);
        assert_eq!(SizeToken::GapSm.px_i16(), 8);
        assert_eq!(SizeToken::GapMd.px_i16(), 12);
    }
}
