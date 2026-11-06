use super::Transform2D;
use crate::layout::{AlignItems, Display, FlexWrap, JustifyContent, Overflow};

/// Sets the pixel size of one spacing step in style atoms.
pub(crate) const SPACING_STEP: f32 = 4.0;

pub(crate) const FACTOR_SCALE: f32 = 1000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Px(pub i16);

mod color;
pub use color::{BgGradient, ColorAtom, GradientAxis, GradientStopKey, Rgba8};

/// Identifies a transition preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransitionId(
    /// The preset number.
    pub u16,
);

/// A duration in whole milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MillisU16(
    /// The milliseconds.
    pub u16,
);

/// Selects a shadow depth from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Elevation {
    /// The sm elevation.
    Sm,
    /// The base elevation.
    Base,
    /// The md elevation.
    Md,
    /// The lg elevation.
    Lg,
    /// The xl elevation.
    Xl,
}

/// Holds the resolved paint properties of one node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderStyle {
    /// The background color, if any.
    pub background_color: Option<Rgba8>,
    /// The background gradient, if any.
    pub background_gradient: Option<BgGradient>,
    /// The text color, if any.
    pub text_color: Option<Rgba8>,
    /// The border radius, if any.
    pub border_radius: Option<i16>,
    /// The border width, if any.
    pub border_width: Option<i16>,
    /// The border top, if any.
    pub border_top: Option<i16>,
    /// The border right, if any.
    pub border_right: Option<i16>,
    /// The border bottom, if any.
    pub border_bottom: Option<i16>,
    /// The border left, if any.
    pub border_left: Option<i16>,
    /// The border color, if any.
    pub border_color: Option<Rgba8>,
    /// The shadow, if any.
    pub shadow: Option<Elevation>,
    /// The opacity.
    pub opacity: u16,
    /// The transform.
    pub transform: Transform2D,
}

impl Default for RenderStyle {
    fn default() -> Self {
        Self {
            background_color: None,
            background_gradient: None,
            text_color: None,
            border_radius: None,
            border_width: None,
            border_top: None,
            border_right: None,
            border_bottom: None,
            border_left: None,
            border_color: None,
            shadow: None,
            opacity: 10_000,
            transform: Transform2D::IDENTITY,
        }
    }
}

/// Selects the line drawn with text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextDecoration {
    /// The none text decoration.
    #[default]
    None,
    /// The underline text decoration.
    Underline,
    /// The line through text decoration.
    LineThrough,
}

/// One style property with its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleAtom {
    /// The padding all style atom.
    PaddingAll(i16),
    /// The padding x style atom.
    PaddingX(i16),
    /// The padding y style atom.
    PaddingY(i16),
    /// The padding top style atom.
    PaddingTop(i16),
    /// The padding right style atom.
    PaddingRight(i16),
    /// The padding bottom style atom.
    PaddingBottom(i16),
    /// The padding left style atom.
    PaddingLeft(i16),
    /// The margin all style atom.
    MarginAll(i16),
    /// The margin x style atom.
    MarginX(i16),
    /// The margin y style atom.
    MarginY(i16),
    /// The margin top style atom.
    MarginTop(i16),
    /// The margin right style atom.
    MarginRight(i16),
    /// The margin bottom style atom.
    MarginBottom(i16),
    /// The margin left style atom.
    MarginLeft(i16),
    /// The gap style atom.
    Gap(i16),
    /// The gap x style atom.
    GapX(i16),
    /// The gap y style atom.
    GapY(i16),
    /// The width style atom.
    Width(u8, i16),
    /// The height style atom.
    Height(u8, i16),
    /// The min width style atom.
    MinWidth(u8, i16),
    /// The max width style atom.
    MaxWidth(u8, i16),
    /// The min height style atom.
    MinHeight(u8, i16),
    /// The max height style atom.
    MaxHeight(u8, i16),
    /// The display style atom.
    Display(Display),
    /// The flex direction style atom.
    FlexDirection(crate::layout::FlexDirection),
    /// The flex wrap style atom.
    FlexWrap(FlexWrap),
    /// The align items style atom.
    AlignItems(AlignItems),
    /// The justify content style atom.
    JustifyContent(JustifyContent),
    /// The flex grow style atom.
    FlexGrow(u16),
    /// The flex shrink style atom.
    FlexShrink(u16),
    /// The grid columns style atom.
    GridColumns(u16),
    /// The grid rows style atom.
    GridRows(u16),
    /// The background color style atom.
    BackgroundColor(ColorAtom),
    /// The background gradient style atom.
    BackgroundGradient(BgGradient),
    /// The text color style atom.
    TextColor(ColorAtom),
    /// The bg tone step style atom.
    BgToneStep(u8),
    /// The fg tone step style atom.
    FgToneStep(u8),
    /// The fg on solid style atom.
    FgOnSolid,
    /// The border radius style atom.
    BorderRadius(i16),
    /// The radius style atom.
    Radius(super::Radius),
    /// The border width style atom.
    BorderWidth(i16),
    /// The border top style atom.
    BorderTop(i16),
    /// The border right style atom.
    BorderRight(i16),
    /// The border bottom style atom.
    BorderBottom(i16),
    /// The border left style atom.
    BorderLeft(i16),
    /// The border color style atom.
    BorderColor(ColorAtom),
    /// The border tone step style atom.
    BorderToneStep(u8),
    /// The shadow style atom.
    Shadow(Elevation),
    /// The opacity style atom.
    Opacity(u16),
    /// The transform style atom.
    Transform(Transform2D),
    /// The font size style atom.
    FontSize(i16),
    /// The font weight style atom.
    FontWeight(u16),
    /// The font italic style atom.
    FontItalic(bool),
    /// The font family style atom.
    FontFamily(&'static str),
    /// The line height style atom.
    LineHeight(i16),
    /// The letter spacing style atom.
    LetterSpacing(i16),
    /// The tabular nums style atom.
    TabularNums(bool),
    /// The text decoration style atom.
    TextDecoration(TextDecoration),
    /// The overflow x style atom.
    OverflowX(Overflow),
    /// The overflow y style atom.
    OverflowY(Overflow),
    /// The transition style atom.
    Transition(TransitionId),
    /// The transition duration style atom.
    TransitionDuration(MillisU16),
    /// The transition easing style atom.
    TransitionEase(crate::animation::Easing),
    /// The transition delay style atom.
    TransitionDelay(MillisU16),
    /// The keyframe animation style atom.
    Animation(&'static crate::animation::Keyframes),
    /// The unknown theme ref style atom.
    UnknownThemeRef(&'static str),
}

impl StyleAtom {
    /// Returns the affects layout.
    #[must_use]
    pub const fn affects_layout(self) -> bool {
        matches!(
            self,
            Self::PaddingAll(_)
                | Self::PaddingX(_)
                | Self::PaddingY(_)
                | Self::PaddingTop(_)
                | Self::PaddingRight(_)
                | Self::PaddingBottom(_)
                | Self::PaddingLeft(_)
                | Self::MarginAll(_)
                | Self::MarginX(_)
                | Self::MarginY(_)
                | Self::MarginTop(_)
                | Self::MarginRight(_)
                | Self::MarginBottom(_)
                | Self::MarginLeft(_)
                | Self::Gap(_)
                | Self::GapX(_)
                | Self::GapY(_)
                | Self::Width(_, _)
                | Self::Height(_, _)
                | Self::MinWidth(_, _)
                | Self::MinHeight(_, _)
                | Self::MaxWidth(_, _)
                | Self::MaxHeight(_, _)
                | Self::Display(_)
                | Self::FlexDirection(_)
                | Self::FlexWrap(_)
                | Self::AlignItems(_)
                | Self::JustifyContent(_)
                | Self::FlexGrow(_)
                | Self::FlexShrink(_)
                | Self::GridColumns(_)
                | Self::GridRows(_)
                | Self::BorderWidth(_)
                | Self::BorderTop(_)
                | Self::BorderRight(_)
                | Self::BorderBottom(_)
                | Self::BorderLeft(_)
                | Self::FontSize(_)
                | Self::FontWeight(_)
                | Self::FontItalic(_)
                | Self::FontFamily(_)
                | Self::LineHeight(_)
                | Self::LetterSpacing(_)
                | Self::TabularNums(_)
                | Self::OverflowX(_)
                | Self::OverflowY(_)
        )
    }
}

pub(crate) const ATOM_PX: u8 = 0;
pub(crate) const ATOM_FULL: u8 = 1;
pub(crate) const ATOM_FRACTION: u8 = 2;
pub(crate) const ATOM_AUTO: u8 = 3;

pub(crate) const fn decode_atom(kind: u8, value: i16) -> crate::layout::Dimension {
    match kind {
        ATOM_FULL => crate::layout::Dimension::Percent(1.0),
        ATOM_FRACTION => crate::layout::Dimension::Percent(value as f32 / 10_000.0),
        ATOM_AUTO => crate::layout::Dimension::Auto,
        _ => crate::layout::Dimension::Px(value as f32),
    }
}

pub(crate) const fn width_px(px: i16) -> StyleAtom {
    StyleAtom::Width(ATOM_PX, px)
}

pub(crate) const fn height_px(px: i16) -> StyleAtom {
    StyleAtom::Height(ATOM_PX, px)
}

pub(crate) const fn min_width_px(px: i16) -> StyleAtom {
    StyleAtom::MinWidth(ATOM_PX, px)
}

pub(crate) const fn max_width_px(px: i16) -> StyleAtom {
    StyleAtom::MaxWidth(ATOM_PX, px)
}

pub(crate) const fn min_height_px(px: i16) -> StyleAtom {
    StyleAtom::MinHeight(ATOM_PX, px)
}

pub(crate) const fn max_height_px(px: i16) -> StyleAtom {
    StyleAtom::MaxHeight(ATOM_PX, px)
}
