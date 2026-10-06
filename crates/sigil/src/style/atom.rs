use super::{ColorKey, Transform2D};

pub const SPACING_STEP: f32 = 4.0;

pub(crate) const FACTOR_SCALE: f32 = 1000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Px(pub i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    Px(Px),
    Full,
    Fraction(u16),
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorAtom {
    Rgba8(Rgba8),
    ThemeRef(ColorKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GradientAxis {
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GradientStopKey {
    pub offset: u16,
    pub color: ColorAtom,
}

impl GradientStopKey {
    #[must_use]
    pub const fn new(offset: u16, color: ColorAtom) -> Self {
        Self { offset, color }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BgGradient {
    pub axis: GradientAxis,
    pub stops: &'static [GradientStopKey],
}

impl BgGradient {
    #[must_use]
    pub const fn new(axis: GradientAxis, stops: &'static [GradientStopKey]) -> Self {
        Self { axis, stops }
    }

    #[must_use]
    pub fn owned(axis: GradientAxis, stops: Vec<GradientStopKey>) -> Self {
        Self::new(axis, intern_stops(stops))
    }
}

fn intern_stops(stops: Vec<GradientStopKey>) -> &'static [GradientStopKey] {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock, PoisonError};

    static CACHE: OnceLock<Mutex<HashSet<&'static [GradientStopKey]>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(interned) = cache.get(stops.as_slice()).copied() {
        return interned;
    }
    let interned: &'static [GradientStopKey] = Box::leak(stops.into_boxed_slice());
    cache.insert(interned);
    interned
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayKind {
    Flex,
    Block,
    Grid,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlexDir {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignKind {
    Start,
    Center,
    End,
    Stretch,
    Between,
    Around,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JustifyKind {
    Start,
    Center,
    End,
    Stretch,
    Between,
    Around,
    Evenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlexWrapKind {
    NoWrap,
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverflowKind {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransitionId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MillisU16(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Elevation {
    Sm,
    Base,
    Md,
    Lg,
    Xl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderStyle {
    pub background_color: Option<Rgba8>,
    pub background_gradient: Option<BgGradient>,
    pub text_color: Option<Rgba8>,
    pub border_radius: Option<Px>,
    pub border_width: Option<Px>,
    pub border_top: Option<Px>,
    pub border_right: Option<Px>,
    pub border_bottom: Option<Px>,
    pub border_left: Option<Px>,
    pub border_color: Option<Rgba8>,
    pub shadow: Option<Elevation>,
    pub opacity: u16,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    LineThrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleAtom {
    PaddingAll(Px),
    PaddingX(Px),
    PaddingY(Px),
    PaddingTop(Px),
    PaddingRight(Px),
    PaddingBottom(Px),
    PaddingLeft(Px),
    MarginAll(Px),
    MarginX(Px),
    MarginY(Px),
    MarginTop(Px),
    MarginRight(Px),
    MarginBottom(Px),
    MarginLeft(Px),
    Gap(Px),
    GapX(Px),
    GapY(Px),
    Width(Dimension),
    Height(Dimension),
    MinWidth(Dimension),
    MaxWidth(Dimension),
    MinHeight(Dimension),
    MaxHeight(Dimension),
    Display(DisplayKind),
    FlexDirection(FlexDir),
    FlexWrap(FlexWrapKind),
    AlignItems(AlignKind),
    JustifyContent(JustifyKind),
    FlexGrow(u16),
    FlexShrink(u16),
    GridColumns(u16),
    GridRows(u16),
    BackgroundColor(ColorAtom),
    BackgroundThemeRef(ColorKey),
    BackgroundGradient(BgGradient),
    TextColor(ColorAtom),
    TextThemeRef(ColorKey),
    BgToneStep(u8),
    FgToneStep(u8),
    FgOnSolid,
    BorderRadius(Px),
    Radius(super::Radius),
    BorderWidth(Px),
    BorderTop(Px),
    BorderRight(Px),
    BorderBottom(Px),
    BorderLeft(Px),
    BorderColor(ColorAtom),
    BorderThemeRef(ColorKey),
    BorderToneStep(u8),
    Shadow(Elevation),
    Opacity(u16),
    Transform(Transform2D),
    FontSize(Px),
    FontWeight(u16),
    FontItalic(bool),
    FontFamily(&'static str),
    LineHeight(Px),
    LetterSpacing(i16),
    TabularNums(bool),
    TextDecoration(TextDecoration),
    OverflowX(OverflowKind),
    OverflowY(OverflowKind),
    Transition(TransitionId),
    TransitionDuration(MillisU16),
    UnknownThemeRef(&'static str),
}

impl StyleAtom {
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
                | Self::Width(_)
                | Self::Height(_)
                | Self::MinWidth(_)
                | Self::MinHeight(_)
                | Self::MaxWidth(_)
                | Self::MaxHeight(_)
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
