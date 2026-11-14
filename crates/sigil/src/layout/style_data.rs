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
pub enum Length {
    /// A fixed length in pixels.
    Px(f32),
    /// A fraction of the containing box, where 1.0 is all of it.
    Percent(f32),
    /// No set length, so the layout derives one; the default.
    #[default]
    Auto,
    /// The full length of the containing box.
    Fill,
    /// The height of this many text lines.
    Rows(f32),
    /// The width of this many character cells.
    Cols(f32),
    /// This many times the font size.
    Em(f32),
}

impl Length {
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

impl From<Length> for LengthPercentageAuto {
    fn from(value: Length) -> Self {
        match value {
            Length::Auto | Length::Fill => Self::Auto,
            Length::Px(px) => Self::Px(px),
            Length::Percent(pct) => Self::Percent(pct),
            Length::Rows(count) => Self::Px(TextLength::Rows(count).px()),
            Length::Cols(count) => Self::Px(TextLength::Cols(count).px()),
            Length::Em(count) => Self::Px(TextLength::Em(count).px()),
        }
    }
}

impl From<f32> for Length {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
}

#[doc(hidden)]
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

#[doc(hidden)]
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

    pub(crate) fn map<U: Copy>(self, f: impl Fn(T) -> U) -> Edges<U> {
        Edges {
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
    /// The node lays out its children in block flow; the default.
    #[default]
    Block,
    /// The node lays out its children as a flex box.
    Flex,
    /// The node lays out its children on a grid.
    Grid,
    /// The node is left out of layout.
    None,
}

/// Overflow mode.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Overflow {
    /// Content can extend past the box and is not clipped; the default.
    #[default]
    Visible,
    /// Content outside the box is clipped.
    Hidden,
    /// Content outside the box is clipped; the CSS `scroll` value.
    Scroll,
    /// Content outside the box is clipped; the CSS `auto` value.
    Auto,
}

/// Positioning mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum Position {
    /// The node keeps its place in the flow of its parent; the default.
    #[default]
    Relative,
    /// The node leaves the flow and is placed by its insets.
    Absolute,
    /// The node keeps its place in the flow and is held at its insets from the edges of the nearest scroll area while its parent is in view.
    Sticky,
}

/// Main axis of a flex box.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FlexDirection {
    /// Children run in a row from the start edge; the default.
    #[default]
    Row,
    /// Children run in a column from the top.
    Column,
    /// Children run in a row in reverse order.
    RowReverse,
    /// Children run in a column in reverse order.
    ColumnReverse,
}

/// Wrap mode of a flex box.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FlexWrap {
    /// All children stay on one line; the default.
    #[default]
    NoWrap,
    /// Children that do not fit move to a new line.
    Wrap,
    /// Children wrap, and new lines stack toward the start of the cross axis.
    WrapReverse,
}

/// Main-axis distribution of children.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JustifyContent {
    /// Children pack at the start of the main axis.
    Start,
    /// Children pack at the end of the main axis.
    End,
    /// Children pack in the middle of the main axis.
    Center,
    /// Children pack at the start of the main axis, the same as `Start`.
    Stretch,
    /// Free space goes between children, with none at the two ends.
    SpaceBetween,
    /// Each child gets equal free space on both sides, so each end gets half a gap.
    SpaceAround,
    /// Free space is equal between children and at the two ends.
    SpaceEvenly,
}

/// Cross-axis alignment of children.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlignItems {
    /// Children sit at the start of the cross axis.
    Start,
    /// Children sit at the end of the cross axis.
    End,
    /// Children sit in the middle of the cross axis.
    Center,
    /// Children sit at the start of the cross axis, the same as `Start`.
    Baseline,
    /// Children grow to the cross size of their line; used when no alignment is set.
    Stretch,
}

/// How lines or tracks spread across the cross axis.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlignContent {
    /// Lines pack at the start of the cross axis.
    #[cfg(feature = "kittest")]
    Start,
    /// Lines pack at the end of the cross axis.
    #[cfg(feature = "kittest")]
    End,
    /// Lines pack in the middle of the cross axis.
    #[cfg(feature = "kittest")]
    Center,
    /// Each line grows by an equal share of the free space on the cross axis.
    Stretch,
    /// Free space goes between lines, with none at the two ends.
    #[cfg(feature = "kittest")]
    SpaceBetween,
    /// Each line gets equal free space on both sides, so each end gets half a gap.
    #[cfg(feature = "kittest")]
    SpaceAround,
    /// Free space is equal between lines and at the two ends.
    #[cfg(feature = "kittest")]
    SpaceEvenly,
}

/// Whether a set size includes padding and border.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BoxSizing {
    /// A set size covers the content only, and padding and border add to it; the default.
    #[default]
    ContentBox,
    /// A set size covers the content, padding, and border.
    BorderBox,
}

/// Size of a grid track.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum TrackSize {
    /// A fixed track size in pixels.
    Px(f32),
    /// A share of the free space, in proportion to this weight.
    Fr(f32),
    /// One share of the free space, the same weight as `Fr(1.0)`.
    Auto,
    /// A minimum size in pixels, then a share weight for the free space.
    MinMax(f32, f32),
}

#[doc(hidden)]
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
    pub flex_basis: Length,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub align_content: Option<AlignContent>,
    pub align_self: Option<AlignItems>,
    pub gap: Axes<Length>,
    pub margin: Edges<Length>,
    pub inset: Edges<Length>,
    pub padding: Edges<Length>,
    pub border: Edges<Length>,
    pub size: Axes<Length>,
    pub min_size: Axes<Length>,
    pub max_size: Axes<Length>,
    pub aspect_ratio: Option<f32>,
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

impl StyleData {
    pub(crate) const DEFAULT: Self = Self {
        display: Display::Block,
        position: Position::Relative,
        overflow: Axes::splat(Overflow::Visible),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::NoWrap,
        flex_grow: 0.0,
        flex_shrink: 1.0,
        stretch_width: false,
        flex_basis: Length::Auto,
        justify_content: None,
        align_items: None,
        align_content: None,
        align_self: None,
        gap: Axes::splat(Length::Px(0.0)),
        margin: Edges::splat(Length::Px(0.0)),
        inset: Edges::splat(Length::Auto),
        padding: Edges::splat(Length::Px(0.0)),
        border: Edges::splat(Length::Px(0.0)),
        size: Axes::splat(Length::Auto),
        min_size: Axes::splat(Length::Auto),
        max_size: Axes::splat(Length::Auto),
        aspect_ratio: None,
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
    };
}

impl Default for StyleData {
    fn default() -> Self {
        Self::DEFAULT
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
