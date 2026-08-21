//! Owned style data for the P0.5 layout engine swap-out.
//!
//! DR-041 seats every style field the block / flex solvers need on a
//! sigil-owned struct so that solvers read only sigil-owned data.
//!
//! The supporting enums and value types are sigil-owned by intent: no
//! upstream layout-engine re-exports. The [`crate::style_adapter`] module
//! applies style atoms directly into `StyleData`.
//!
//! Naming follows [authoring-syntax](../../../docs/architecture/authoring-syntax.md):
//! one-word where possible (`Display::Block`, `Position::Absolute`).
//! Multi-word variants survive only where the CSS spec mandates them
//! (`JustifyContent::SpaceBetween`).

/// Axis-independent length expressed as logical pixels or a percentage
/// of the containing block on the matching axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LengthPercentage {
    /// Logical pixels.
    Px(f32),
    /// Fraction in `[0.0, 1.0]` of the containing block on this axis.
    Percent(f32),
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
    /// Compatibility constructor matching the old length authoring helper.
    #[must_use]
    pub const fn length(value: f32) -> Self {
        Self::Px(value)
    }
}

/// `LengthPercentage` extended with the `auto` keyword (margins, insets).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LengthPercentageAuto {
    /// Logical pixels.
    Px(f32),
    /// Fraction of the containing block.
    Percent(f32),
    /// CSS `auto` — value is resolved by the algorithm.
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
    /// Compatibility constructor matching the old length authoring helper.
    #[must_use]
    pub const fn length(value: f32) -> Self {
        Self::Px(value)
    }
}

/// Single-axis size on a CSS-style box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    /// Logical pixels.
    Px(f32),
    /// Fraction of the containing block.
    Percent(f32),
    /// CSS `auto` — fall back to content size.
    #[default]
    Auto,
    /// Inherit the parent's resolved content-box on this axis.
    ///
    /// Equivalent in math to `Percent(1.0)` but expresses intent
    /// directly: "fill whatever the parent gives me". Resolves against
    /// the containing-block size during the layout pass, so a child
    /// inside a parent of 800×600 with padding(20) resolves to 760×560
    /// (the parent's content box, which is its children's containing
    /// block).
    Fill,
}

impl From<f32> for Dimension {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
}

/// Two-axis pair (`width` / `height` or `x` / `y`).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size<T: Copy> {
    /// Inline axis (width).
    pub width: T,
    /// Block axis (height).
    pub height: T,
}

impl<T: Copy> Size<T> {
    /// Build a size from explicit axes.
    #[must_use]
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }

    /// Uniform — both axes share the same value.
    #[must_use]
    pub const fn splat(value: T) -> Self {
        Self {
            width: value,
            height: value,
        }
    }
}

/// Four-sided rect (`left` / `right` / `top` / `bottom`).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect<T: Copy> {
    /// Left edge.
    pub left: T,
    /// Right edge.
    pub right: T,
    /// Top edge.
    pub top: T,
    /// Bottom edge.
    pub bottom: T,
}

impl<T: Copy> Rect<T> {
    /// Uniform — same value on every side.
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

/// Layout algorithm selector.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    /// Block layout (single-axis stacking).
    #[default]
    Block,
    /// Flex layout (one-dimensional with grow / shrink).
    Flex,
    /// Grid layout (two-dimensional tracks).
    Grid,
    /// Hidden from layout entirely.
    None,
}

/// Overflow behavior for a single axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Overflow {
    /// Content extends past the box without clipping (default).
    #[default]
    Visible,
    /// Content past the box is clipped at the border edge.
    Hidden,
    /// Clip to the box. A scrollport (wheel / keys / thumb) is the
    /// same job as CSS `overflow: auto` — `scroll()` is sugar for this
    /// on Y today.
    Scroll,
}

/// Positioning scheme — `Relative` participates in the flow, `Absolute`
/// is taken out of flow and placed against its containing block.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Position {
    /// In-flow (default).
    #[default]
    Relative,
    /// Out-of-flow, placed by `inset` against its containing block.
    Absolute,
}

/// Flex main-axis direction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexDirection {
    /// Main axis runs inline (default).
    #[default]
    Row,
    /// Main axis runs in the block direction.
    Column,
    /// Inline-reversed.
    RowReverse,
    /// Block-reversed.
    ColumnReverse,
}

/// Flex line-wrap mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexWrap {
    /// Single-line (default).
    #[default]
    NoWrap,
    /// Wrap onto additional lines.
    Wrap,
    /// Wrap with reversed cross-axis order.
    WrapReverse,
}

/// Main-axis content distribution for flex / grid containers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JustifyContent {
    /// Pack items at the main-start.
    Start,
    /// Pack items at the main-end.
    End,
    /// Center items on the main axis.
    Center,
    /// Stretch items to fill the main axis.
    Stretch,
    /// Equal space between items, none at the edges.
    SpaceBetween,
    /// Equal space around each item.
    SpaceAround,
    /// Equal space between every pair of adjacent items including edges.
    SpaceEvenly,
}

/// Cross-axis child alignment (`align-items` / `align-self`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlignItems {
    /// Align to the cross-start edge.
    Start,
    /// Align to the cross-end edge.
    End,
    /// Center on the cross axis.
    Center,
    /// Align child baselines.
    Baseline,
    /// Stretch to fill the cross axis.
    Stretch,
}

#[allow(non_upper_case_globals)]
impl AlignItems {
    /// Compatibility alias for the legacy variant name.
    pub const FlexStart: Self = Self::Start;
    /// Compatibility alias for the legacy variant name.
    pub const FlexEnd: Self = Self::End;
}

/// Per-line cross-axis distribution (`align-content`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlignContent {
    /// Pack lines at the cross-start.
    Start,
    /// Pack lines at the cross-end.
    End,
    /// Center lines on the cross axis.
    Center,
    /// Stretch lines to fill the cross axis.
    Stretch,
    /// Equal space between lines.
    SpaceBetween,
    /// Equal space around each line.
    SpaceAround,
    /// Equal space between every pair of adjacent lines including edges.
    SpaceEvenly,
}

/// Self-override of the parent container's `align-items` for one child.
pub type AlignSelf = AlignItems;

/// Whether `size` includes or excludes border + padding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BoxSizing {
    /// `size` is content-area only (CSS default).
    #[default]
    ContentBox,
    /// `size` includes padding and border.
    BorderBox,
}

/// Owned style atom consumed by the P0.5 owned layout engine.
///
/// Mirrors the layout fields the sigil widget code and solvers actually
/// One column (or row) track size in a grid template.
///
/// The minimal grid supports the three common track kinds: a fixed pixel
/// width, a flexible `fr` fraction of the leftover space, and `auto`
/// (sized to the widest cell in the track). No `minmax()`, spanning, or
/// named lines yet — see the private `grid` module.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackSize {
    /// Fixed track size in logical pixels.
    Px(f32),
    /// A fraction of the free space left after fixed + auto tracks.
    Fr(f32),
    /// Sized to the widest cell placed in this track.
    Auto,
    /// `minmax(min_px, fr)`: flexes like `Fr(fr)` but never shrinks below
    /// `min_px`. The common responsive-column shape, e.g.
    /// `minmax(100.0, 1.0)` for a 1fr column that stays at least 100px.
    MinMax(f32, f32),
}

/// Renderer-agnostic resolved layout inputs for one node.
///
/// Held by value on the drive so the hot path clones cheaply and the
/// cache can diff by value. No `Copy` because
/// [`StyleData::grid_template_columns`] is a `Vec` (empty and
/// allocation-free for the common non-grid node).
#[derive(Clone, Debug, PartialEq)]
pub struct StyleData {
    /// Layout algorithm selector.
    pub display: Display,
    /// In-flow vs out-of-flow positioning.
    pub position: Position,
    /// Overflow clipping / scroll behavior per axis.
    pub overflow: Size<Overflow>,
    /// Flex main-axis direction.
    pub flex_direction: FlexDirection,
    /// Flex line-wrap mode.
    pub flex_wrap: FlexWrap,
    /// Flex grow factor.
    pub flex_grow: f32,
    /// Flex shrink factor.
    pub flex_shrink: f32,
    /// Initial flex item main-size before grow / shrink.
    pub flex_basis: Dimension,
    /// Main-axis content distribution.
    pub justify_content: Option<JustifyContent>,
    /// Cross-axis child alignment on the container.
    pub align_items: Option<AlignItems>,
    /// Per-line cross-axis distribution.
    pub align_content: Option<AlignContent>,
    /// Self-override of the parent's `align_items`.
    pub align_self: Option<AlignSelf>,
    /// Gap between flex / grid items on both axes.
    pub gap: Size<LengthPercentage>,
    /// Margin (`auto`-capable).
    pub margin: Rect<LengthPercentageAuto>,
    /// Absolute-position offsets (`auto`-capable).
    pub inset: Rect<LengthPercentageAuto>,
    /// Padding (non-`auto`).
    pub padding: Rect<LengthPercentage>,
    /// Border thickness (non-`auto`).
    pub border: Rect<LengthPercentage>,
    /// Preferred width / height.
    pub size: Size<Dimension>,
    /// Minimum width / height.
    pub min_size: Size<Dimension>,
    /// Maximum width / height.
    pub max_size: Size<Dimension>,
    /// Sort order for flex items.
    pub order: i32,
    /// Whether `size` includes padding + border.
    pub box_sizing: BoxSizing,
    /// Column tracks for `display: grid`. Empty on non-grid nodes.
    pub grid_template_columns: Vec<TrackSize>,
    /// Explicit row tracks for `display: grid`. Empty means every row is
    /// auto (sized to its tallest cell); a `Px` track pins that row's
    /// height, `Auto`/`Fr` fall back to the measured height. Rows past the
    /// template length are implicit and auto-sized.
    pub grid_template_rows: Vec<TrackSize>,
    /// Number of columns this node spans as a grid item (default 1). Read
    /// by the parent grid's auto-flow placement; ignored elsewhere.
    pub grid_column_span: u16,
    /// Number of rows this node spans as a grid item (default 1). The
    /// auto-flow reserves a `column_span × row_span` block and later items
    /// flow around it.
    pub grid_row_span: u16,
    /// Explicit 1-based column line to place this grid item at (`0` = auto).
    /// When both starts are set the item is placed there and auto items
    /// flow around it.
    pub grid_column_start: u16,
    /// Explicit 1-based row line to place this grid item at (`0` = auto).
    pub grid_row_start: u16,
    /// `grid-auto-flow: dense` on a grid CONTAINER (default `false` =
    /// sparse). When set, each auto-placed item restarts its search from
    /// the grid origin, backfilling gaps that a spanning or explicitly-
    /// placed item left earlier, instead of only moving the cursor
    /// forward. Read on the container; ignored on items.
    pub grid_auto_flow_dense: bool,
}

impl Default for StyleData {
    fn default() -> Self {
        // CSS defaults — display Block, position Relative, no grow, no
        // shrink baseline of 1.0, auto size, zero margins / paddings.
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
            grid_auto_flow_dense: false,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::cognitive_complexity)]
#[path = "style_data_tests.rs"]
mod tests;
