use crate::layout::units::TextLength;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LengthPercentage {
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

impl LengthPercentage {
    #[must_use]
    pub const fn length(value: f32) -> Self {
        Self::Px(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LengthPercentageAuto {
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

impl LengthPercentageAuto {
    #[must_use]
    pub const fn length(value: f32) -> Self {
        Self::Px(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    Px(f32),
    Percent(f32),
    #[default]
    Auto,
    Fill,
    Text(TextLength),
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
pub struct Size<T: Copy> {
    pub width: T,
    pub height: T,
}

impl<T: Copy> Size<T> {
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
}

#[allow(
    clippy::derive_partial_eq_without_eq,
    reason = "intentional, reviewed trade-off"
)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect<T: Copy> {
    pub left: T,
    pub right: T,
    pub top: T,
    pub bottom: T,
}

impl<T: Copy> Rect<T> {
    #[must_use]
    pub const fn splat(value: T) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    #[default]
    Block,
    Flex,
    Grid,
    None,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JustifyContent {
    Start,
    End,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlignItems {
    Start,
    End,
    Center,
    Baseline,
    Stretch,
}

#[allow(non_upper_case_globals, reason = "intentional, reviewed trade-off")]
impl AlignItems {
    pub const FlexStart: Self = Self::Start;
    pub const FlexEnd: Self = Self::End;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlignContent {
    Start,
    End,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

pub type AlignSelf = AlignItems;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackSize {
    Px(f32),
    Fr(f32),
    Auto,
    MinMax(f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleData {
    pub display: Display,
    pub position: Position,
    pub overflow: Size<Overflow>,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub align_content: Option<AlignContent>,
    pub align_self: Option<AlignSelf>,
    pub gap: Size<LengthPercentage>,
    pub margin: Rect<LengthPercentageAuto>,
    pub inset: Rect<LengthPercentageAuto>,
    pub padding: Rect<LengthPercentage>,
    pub border: Rect<LengthPercentage>,
    pub size: Size<Dimension>,
    pub min_size: Size<Dimension>,
    pub max_size: Size<Dimension>,
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
            overflow: Size::splat(Overflow::Visible),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            justify_content: None,
            align_items: None,
            align_content: None,
            align_self: None,
            gap: Size::splat(LengthPercentage::Px(0.0)),
            margin: Rect::splat(LengthPercentageAuto::Px(0.0)),
            inset: Rect::splat(LengthPercentageAuto::Auto),
            padding: Rect::splat(LengthPercentage::Px(0.0)),
            border: Rect::splat(LengthPercentage::Px(0.0)),
            size: Size::splat(Dimension::Auto),
            min_size: Size::splat(Dimension::Auto),
            max_size: Size::splat(Dimension::Auto),
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
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "style_data_tests.rs"]
mod tests;
