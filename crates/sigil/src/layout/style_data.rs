use crate::layout::units::TextLength;
use crate::style::Px;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LengthPercentage {
    Px(f32),
    Percent(f32),
    Text(TextLength),
}

impl Default for LengthPercentage {
    fn default() -> Self {
        Self::Px(0.0)
    }
}

impl From<f32> for LengthPercentage {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
}

impl From<Px> for LengthPercentage {
    fn from(px: Px) -> Self {
        Self::Px(f32::from(px.0))
    }
}

impl From<i16> for LengthPercentage {
    fn from(px: i16) -> Self {
        Self::Px(f32::from(px))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LengthPercentageAuto {
    Px(f32),
    Percent(f32),
    Auto,
}

impl Default for LengthPercentageAuto {
    fn default() -> Self {
        Self::Px(0.0)
    }
}

impl From<f32> for LengthPercentageAuto {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
}

/// The length of a size, padding, margin, gap, or inset.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum Dimension {
    /// The px dimension.
    Px(f32),
    /// The percent dimension.
    Percent(f32),
    /// The auto dimension.
    #[default]
    Auto,
    /// The fill dimension.
    Fill,
    /// The rows dimension.
    Rows(f32),
    /// The cols dimension.
    Cols(f32),
    /// The em dimension.
    Em(f32),
}

impl Dimension {
    pub(crate) const fn definite(self) -> Option<LengthPercentage> {
        match self {
            Self::Px(px) => Some(LengthPercentage::Px(px)),
            Self::Percent(pct) => Some(LengthPercentage::Percent(pct)),
            Self::Rows(count) => Some(LengthPercentage::Text(TextLength::Rows(count))),
            Self::Cols(count) => Some(LengthPercentage::Text(TextLength::Cols(count))),
            Self::Em(count) => Some(LengthPercentage::Text(TextLength::Em(count))),
            Self::Auto | Self::Fill => None,
        }
    }
}

impl From<Dimension> for LengthPercentageAuto {
    fn from(value: Dimension) -> Self {
        match value {
            Dimension::Auto | Dimension::Fill => Self::Auto,
            Dimension::Px(px) => Self::Px(px),
            Dimension::Percent(pct) => Self::Percent(pct),
            Dimension::Rows(count) => Self::Px(TextLength::Rows(count).px()),
            Dimension::Cols(count) => Self::Px(TextLength::Cols(count).px()),
            Dimension::Em(count) => Self::Px(TextLength::Em(count).px()),
        }
    }
}

impl From<f32> for Dimension {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
}

#[allow(
    clippy::derive_partial_eq_without_eq,
    reason = "intentional, reviewed trade-off"
)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Axes<T: Copy> {
    pub width: T,
    pub height: T,
}

impl<T: Copy> Axes<T> {
    #[must_use]
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn splat(value: T) -> Self {
        Self {
            width: value,
            height: value,
        }
    }

    pub(crate) fn map(self, f: impl Fn(T) -> T) -> Self {
        Self::new(f(self.width), f(self.height))
    }
}

#[allow(
    clippy::derive_partial_eq_without_eq,
    reason = "intentional, reviewed trade-off"
)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges<T: Copy> {
    pub left: T,
    pub right: T,
    pub top: T,
    pub bottom: T,
}

impl<T: Copy> Edges<T> {
    #[must_use]
    pub const fn splat(value: T) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    pub(crate) fn map(self, f: impl Fn(T) -> T) -> Self {
        Self {
            left: f(self.left),
            right: f(self.right),
            top: f(self.top),
            bottom: f(self.bottom),
        }
    }
}

/// Display mode.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Display {
    /// The block display.
    #[default]
    Block,
    /// The flex display.
    Flex,
    /// The grid display.
    Grid,
    /// The none display.
    None,
}

/// Overflow mode.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Overflow {
    /// The visible overflow.
    #[default]
    Visible,
    /// The hidden overflow.
    Hidden,
    /// The scroll overflow.
    Scroll,
    /// The auto overflow.
    Auto,
}

/// Positioning mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum Position {
    /// The relative position.
    #[default]
    Relative,
    /// The absolute position.
    Absolute,
}

/// Main axis of a flex box.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FlexDirection {
    /// The row flex direction.
    #[default]
    Row,
    /// The column flex direction.
    Column,
    /// The row reverse flex direction.
    RowReverse,
    /// The column reverse flex direction.
    ColumnReverse,
}

/// Wrap mode of a flex box.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FlexWrap {
    /// The no wrap flex wrap.
    #[default]
    NoWrap,
    /// The wrap flex wrap.
    Wrap,
    /// The wrap reverse flex wrap.
    WrapReverse,
}

/// Main-axis distribution of children.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JustifyContent {
    /// The start justify content.
    Start,
    /// The end justify content.
    End,
    /// The center justify content.
    Center,
    /// The stretch justify content.
    Stretch,
    /// The space between justify content.
    SpaceBetween,
    /// The space around justify content.
    SpaceAround,
    /// The space evenly justify content.
    SpaceEvenly,
}

/// Cross-axis alignment of children.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlignItems {
    /// The start align items.
    Start,
    /// The end align items.
    End,
    /// The center align items.
    Center,
    /// The baseline align items.
    Baseline,
    /// The stretch align items.
    Stretch,
}

/// How lines or tracks spread across the cross axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlignContent {
    /// The start align content.
    #[cfg(any(test, feature = "kittest"))]
    Start,
    /// The end align content.
    #[cfg(any(test, feature = "kittest"))]
    End,
    /// The center align content.
    #[cfg(any(test, feature = "kittest"))]
    Center,
    /// The stretch align content.
    Stretch,
    /// The space between align content.
    #[cfg(any(test, feature = "kittest"))]
    SpaceBetween,
    /// The space around align content.
    #[cfg(any(test, feature = "kittest"))]
    SpaceAround,
    /// The space evenly align content.
    #[cfg(any(test, feature = "kittest"))]
    SpaceEvenly,
}

/// Whether a set size includes padding and border.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BoxSizing {
    /// The content box box sizing.
    #[default]
    ContentBox,
    /// The border box box sizing.
    BorderBox,
}

/// Size of a grid track.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum TrackSize {
    /// The px track size.
    Px(f32),
    /// The fr track size.
    Fr(f32),
    /// The auto track size.
    Auto,
    /// The min max track size.
    MinMax(f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleData {
    pub display: Display,
    pub position: Position,
    pub overflow: Axes<Overflow>,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub stretch_width: bool,
    pub flex_basis: Dimension,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub align_content: Option<AlignContent>,
    pub align_self: Option<AlignItems>,
    pub gap: Axes<Dimension>,
    pub margin: Edges<Dimension>,
    pub inset: Edges<Dimension>,
    pub padding: Edges<Dimension>,
    pub border: Edges<Dimension>,
    pub size: Axes<Dimension>,
    pub min_size: Axes<Dimension>,
    pub max_size: Axes<Dimension>,
    pub order: i32,
    pub box_sizing: BoxSizing,
    pub grid_template_columns: Vec<TrackSize>,
    pub grid_template_rows: Vec<TrackSize>,
    pub grid_column_span: u16,
    pub grid_row_span: u16,
    pub grid_column_start: u16,
    pub grid_row_start: u16,
    pub grid_auto_flow_dense: bool,
    pub direction: crate::style::Direction,
}

impl Default for StyleData {
    fn default() -> Self {
        Self {
            display: Display::Block,
            position: Position::Relative,
            overflow: Axes::splat(Overflow::Visible),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            stretch_width: false,
            flex_basis: Dimension::Auto,
            justify_content: None,
            align_items: None,
            align_content: None,
            align_self: None,
            gap: Axes::splat(Dimension::Px(0.0)),
            margin: Edges::splat(Dimension::Px(0.0)),
            inset: Edges::splat(Dimension::Auto),
            padding: Edges::splat(Dimension::Px(0.0)),
            border: Edges::splat(Dimension::Px(0.0)),
            size: Axes::splat(Dimension::Auto),
            min_size: Axes::splat(Dimension::Auto),
            max_size: Axes::splat(Dimension::Auto),
            order: 0,
            box_sizing: BoxSizing::ContentBox,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_column_span: 1,
            grid_row_span: 1,
            grid_column_start: 0,
            grid_row_start: 0,
            direction: crate::style::Direction::Ltr,
            grid_auto_flow_dense: false,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cognitive_complexity,
    clippy::float_cmp,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "style_data_tests.rs"]
mod tests;
