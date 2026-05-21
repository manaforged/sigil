//! Owned style data for the P0.5 layout engine swap-out.
//!
//! [DR-041](../../../docs/decisions/DR-041-owned-layout-engine.md) seats
//! every style field the block / flex solvers need on a sigil-owned
//! struct so that solvers read only sigil-owned data.
//!
//! The supporting enums and value types are sigil-owned by intent: no
//! upstream layout-engine re-exports. The [`crate::style_adapter`] module
//! applies style atoms directly into `StyleData`.
//!
//! Naming follows [authoring-syntax](../../../docs/architecture/authoring-syntax.md):
//! one-word where possible (`Display::Block`, `Position::Absolute`).
//! Multi-word variants survive only where the CSS spec mandates them
//! (`JustifyContent::SpaceBetween`).

#![allow(dead_code)]

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
    /// Same as `Hidden` but reserves space for a scrollbar (renderer-coupled).
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
/// read. Plain Rust — no `Copy` because nested enums
/// preclude it; cheap to `Clone` (every field is `Copy`).
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
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::cognitive_complexity)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_css_baseline() {
        let s = StyleData::default();
        assert_eq!(s.display, Display::Block);
        assert_eq!(s.position, Position::Relative);
        assert_eq!(s.flex_direction, FlexDirection::Row);
        assert_eq!(s.flex_wrap, FlexWrap::NoWrap);
        assert!((s.flex_grow - 0.0).abs() < f32::EPSILON);
        assert!((s.flex_shrink - 1.0).abs() < f32::EPSILON);
        assert_eq!(s.flex_basis, Dimension::Auto);
        assert_eq!(s.justify_content, None);
        assert_eq!(s.align_items, None);
        assert_eq!(s.align_content, None);
        assert_eq!(s.align_self, None);
        assert_eq!(s.overflow.width, Overflow::Visible);
        assert_eq!(s.overflow.height, Overflow::Visible);
        assert_eq!(s.inset.left, LengthPercentageAuto::Auto);
        assert_eq!(s.inset.right, LengthPercentageAuto::Auto);
        assert_eq!(s.inset.top, LengthPercentageAuto::Auto);
        assert_eq!(s.inset.bottom, LengthPercentageAuto::Auto);
        assert_eq!(s.size.width, Dimension::Auto);
        assert_eq!(s.size.height, Dimension::Auto);
        assert_eq!(s.box_sizing, BoxSizing::ContentBox);
    }

    #[test]
    fn size_and_rect_splat_fill_every_field() {
        let r: Rect<f32> = Rect::splat(7.0);
        assert!((r.left - 7.0).abs() < f32::EPSILON);
        assert!((r.right - 7.0).abs() < f32::EPSILON);
        assert!((r.top - 7.0).abs() < f32::EPSILON);
        assert!((r.bottom - 7.0).abs() < f32::EPSILON);
        let s: Size<i32> = Size::splat(3);
        assert_eq!(s.width, 3);
        assert_eq!(s.height, 3);
    }

    #[test]
    fn style_data_is_cloneable_and_eq_aware() {
        let a = StyleData::default();
        let b = a.clone();
        assert_eq!(a, b);
        let mut c = a.clone();
        c.flex_grow = 1.0;
        assert_ne!(a, c);
    }
}
