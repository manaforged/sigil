use super::Transform2D;
use crate::layout::{AlignItems, Display, FlexWrap, JustifyContent, Overflow};

/// Sets the pixel size of one spacing step in style atoms.
pub(crate) const SPACING_STEP: f32 = 4.0;

pub(crate) const FACTOR_SCALE: f32 = 1000.0;

pub(crate) const OPACITY_UNITS: f32 = 10_000.0;

pub(crate) const fn bits(value: f32) -> u32 {
    if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        (value + 0.0).to_bits()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Px(pub i16);

mod background;
#[cfg(test)]
pub(crate) use background::NO_GRADIENT;
mod clip;
mod color;
mod filter;
mod gradient;
mod mask;
mod space;
pub use background::{Background, Gradient};
pub use clip::ClipShape;
pub use color::{Color, ColorValue, GradientAxis};
pub(crate) use color::direction;
kittest_pub! {
    #[doc(hidden)]
    pub use filter::FilterAtom;
}
pub(crate) use filter::{
    Amount, BackdropPart, FilterPart, FilterStyle, ShadowSpec, TextShadows, aligned, blur_units,
    hue_units, shadow_color,
};
kittest_pub! {
    #[doc(hidden)]
    pub use gradient::{BorderStyle, GradientShape, PaintAtom, PaintStyle, StopSlot};
}
pub use gradient::{Conic, Radial, RadialExtent};
pub(crate) use gradient::Geometry;
kittest_pub! {
    #[doc(hidden)]
    pub use mask::MaskAtom;
}
pub use mask::{MaskImage, MaskSize};
pub(crate) use mask::{MaskKind, MaskPart, MaskStyle};
kittest_pub! {
    #[doc(hidden)]
    pub use space::{Backface, SpaceAtom, SpaceStyle};
}

kittest_pub! {
    #[doc(hidden)]
    /// Identifies a transition preset.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TransitionId(
        /// The number that names the preset, such as 0 for the smooth preset.
        pub u16,
    );
}

kittest_pub! {
    #[doc(hidden)]
    /// A duration in whole milliseconds.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MillisU16(
        /// The duration in milliseconds, from 0 to 65535.
        pub u16,
    );
}

/// Selects a shadow depth from the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Elevation {
    /// The shallowest shadow: 1 px down with a 2 px blur.
    Sm,
    /// A shadow 2 px down with a 4 px blur.
    Base,
    /// A shadow 4 px down with an 8 px blur, the depth that overlays use.
    Md,
    /// A shadow 8 px down with a 16 px blur.
    Lg,
    /// The deepest shadow: 16 px down with a 28 px blur.
    Xl,
}

kittest_pub! {
    /// Holds the resolved paint properties of one node.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RenderStyle {
        /// The solid background color; `None` when a gradient or nothing fills the box.
        pub background_color: Option<Color>,
        /// The text color, when a style atom sets one.
        pub text_color: Option<Color>,
        /// The corner radius in pixels, when a style atom sets one.
        pub border_radius: Option<i16>,
        /// The border width on all four sides in pixels, when a style atom sets one.
        pub border_width: Option<i16>,
        /// The width of the top border in pixels, when set for that side.
        pub border_top: Option<i16>,
        /// The width of the right border in pixels, when set for that side.
        pub border_right: Option<i16>,
        /// The width of the bottom border in pixels, when set for that side.
        pub border_bottom: Option<i16>,
        /// The width of the left border in pixels, when set for that side.
        pub border_left: Option<i16>,
        /// The border color, when a style atom sets one.
        pub border_color: Option<Color>,
        /// The shadow depth, when a style atom sets one.
        pub shadow: Option<Elevation>,
        /// The opacity in ten-thousandths, where 10000 is opaque and the default.
        pub opacity: u16,
        /// The 2D move, scale, and turn applied when the node paints; the default is the identity.
        pub transform: Transform2D,
        /// The gradient, image, clip, and border style paint.
        pub paint: PaintStyle,
        /// The shape that clips the paint of the node, when a style atom sets one.
        pub clip_path: Option<ClipShape>,
        /// The 3D turn, depth, perspective, and backface.
        pub space: SpaceStyle,
        pub(crate) mask: MaskStyle,
        /// The CSS mix-blend-mode.
        pub blend: crate::scene::BlendMode,
        pub(crate) filter: FilterStyle,
    }
}

impl Default for RenderStyle {
    fn default() -> Self {
        Self {
            background_color: None,
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
            paint: PaintStyle {
                gradient: None,
                stops: [None; 3],
                positions: [None; 3],
                clip_text: false,
                image: None,
                border: BorderStyle::Solid,
                geometry: Geometry::default(),
            },
            clip_path: None,
            space: SpaceStyle::FLAT,
            mask: MaskStyle::NONE,
            blend: crate::scene::BlendMode::Normal,
            filter: FilterStyle::NONE,
        }
    }
}

impl RenderStyle {
    pub(crate) fn preserves_3d(&self, clips: bool) -> bool {
        self.space.preserve
            && !clips
            && f32::from(self.opacity) >= OPACITY_UNITS
            && self.blend == crate::scene::BlendMode::Normal
            && self.clip_path.is_none()
            && self.mask.kind.is_none()
            && self.filter.filters().is_empty()
            && self.filter.backdrop().is_empty()
    }
}

/// Selects the line drawn with text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextDecoration {
    /// No line is drawn with the text.
    #[default]
    None,
    /// A line under the text.
    Underline,
    /// A line through the middle of the text.
    LineThrough,
}

kittest_pub! {
    #[doc(hidden)]
    /// One style property with its value.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum StyleAtom {
        /// Padding on all four sides, in pixels.
        PaddingAll(i16),
        /// Padding on the left and right, in pixels.
        PaddingX(i16),
        /// Padding on the top and bottom, in pixels.
        PaddingY(i16),
        /// Padding on the top, in pixels.
        PaddingTop(i16),
        /// Padding on the right, in pixels.
        PaddingRight(i16),
        /// Padding on the bottom, in pixels.
        PaddingBottom(i16),
        /// Padding on the left, in pixels.
        PaddingLeft(i16),
        /// Margin on all four sides, in pixels.
        MarginAll(i16),
        /// Margin on the left and right, in pixels.
        MarginX(i16),
        /// Margin on the top and bottom, in pixels.
        MarginY(i16),
        /// Margin on the top, in pixels.
        MarginTop(i16),
        /// Margin on the right, in pixels.
        MarginRight(i16),
        /// Margin on the bottom, in pixels.
        MarginBottom(i16),
        /// Margin on the left, in pixels.
        MarginLeft(i16),
        /// The gap between children on both axes, in pixels.
        Gap(i16),
        /// The horizontal gap between children, in pixels.
        GapX(i16),
        /// The vertical gap between children, in pixels.
        GapY(i16),
        /// The width as a kind and a value: pixels, the full container, a fraction in ten-thousandths, or auto.
        Width(u8, i16),
        /// The height as a kind and a value: pixels, the full container, a fraction in ten-thousandths, or auto.
        Height(u8, i16),
        /// The minimum width as a kind and a value: pixels, the full container, a fraction in ten-thousandths, or auto.
        MinWidth(u8, i16),
        /// The maximum width as a kind and a value: pixels, the full container, a fraction in ten-thousandths, or auto.
        MaxWidth(u8, i16),
        /// The minimum height as a kind and a value: pixels, the full container, a fraction in ten-thousandths, or auto.
        MinHeight(u8, i16),
        /// The maximum height as a kind and a value: pixels, the full container, a fraction in ten-thousandths, or auto.
        MaxHeight(u8, i16),
        /// The layout mode of the node: block, flex, grid, or none.
        Display(Display),
        /// The main axis of a flex box and the direction children run along it.
        FlexDirection(crate::layout::FlexDirection),
        /// Whether the children of a flex box wrap onto more lines.
        FlexWrap(FlexWrap),
        /// The alignment of children on the cross axis.
        AlignItems(AlignItems),
        /// The distribution of children along the main axis.
        JustifyContent(JustifyContent),
        /// The flex grow factor in thousandths, so 1000 is a factor of 1.
        FlexGrow(u16),
        /// The flex shrink factor in thousandths, so 1000 is a factor of 1.
        FlexShrink(u16),
        /// Makes the node a grid with this many columns of equal width.
        GridColumns(u16),
        /// Makes the node a grid with this many rows.
        GridRows(u16),
        /// A solid background color, which replaces a background gradient.
        BackgroundColor(ColorValue),
        /// The color of the text.
        TextColor(ColorValue),
        /// Fills the background with this step, 1 to 12, of the tone in scope.
        BgToneStep(u8),
        /// Colors the text with this step, 1 to 12, of the tone in scope.
        FgToneStep(u8),
        /// Colors the text with the `on_solid` color of the tone in scope.
        FgOnSolid,
        /// The corner radius in pixels.
        BorderRadius(i16),
        /// The corner radius as a step of the radius scale.
        Radius(super::Radius),
        /// The border width on all four sides, in pixels.
        BorderWidth(i16),
        /// The width of the top border, in pixels.
        BorderTop(i16),
        /// The width of the right border, in pixels.
        BorderRight(i16),
        /// The width of the bottom border, in pixels.
        BorderBottom(i16),
        /// The width of the left border, in pixels.
        BorderLeft(i16),
        /// The color of the border.
        BorderColor(ColorValue),
        /// Colors the border with this step, 1 to 12, of the tone in scope.
        BorderToneStep(u8),
        /// The depth of the shadow that the node casts.
        Shadow(Elevation),
        /// The opacity in ten-thousandths, where 10000 is opaque.
        Opacity(u16),
        /// A 2D move, scale, and turn applied when the node paints.
        #[cfg(feature = "kittest")]
        Transform(Transform2D),
        /// The font size in pixels, at least 1.
        FontSize(i16),
        /// The font weight, clamped to the range 1 to 1000.
        FontWeight(u16),
        /// Whether the text is italic.
        FontItalic(bool),
        /// The name of the font family, such as `sans` or `mono`.
        FontFamily(&'static str),
        /// The line height in pixels, at least 1.
        LineHeight(i16),
        /// The letter spacing in thousandths of an em.
        LetterSpacing(i16),
        /// Whether digits all take the same width.
        TabularNums(bool),
        /// The line drawn with the text: none, underline, or line-through.
        TextDecoration(TextDecoration),
        /// What happens to content that is wider than the node.
        OverflowX(Overflow),
        /// What happens to content that is taller than the node.
        OverflowY(Overflow),
        /// The transition preset of the node, by id.
        Transition(TransitionId),
        /// The duration of the transition, in milliseconds.
        TransitionDuration(MillisU16),
        /// The easing curve of the transition.
        TransitionEase(crate::animation::Easing),
        /// The delay before the transition starts, in milliseconds.
        TransitionDelay(MillisU16),
        /// A keyframe animation, by the id of its keyframes.
        Animation(crate::animation::KeyframesId),
        /// The gradient, image, clip, and border style atom.
        Paint(PaintAtom),
        /// The shape that clips the paint of the node.
        ClipPath(ClipShape),
        /// The 3D transform style atom.
        Space(SpaceAtom),
        /// One part of the mask of the node.
        Mask(MaskAtom),
        /// The mix-blend-mode style atom.
        MixBlend(crate::scene::BlendMode),
        /// The filter and text shadow style atom.
        Filter(FilterAtom),
        /// The color of the drop shadow filter.
        DropShadowColor(ColorValue),
        /// The color of the text shadow.
        TextShadowColor(ColorValue),
    }
}

impl StyleAtom {
    kittest_pub! {
        #[doc(hidden)]
        /// Whether this atom changes layout, such as padding, margin, gap, or size.
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

    #[must_use]
    pub(crate) const fn may_transform(self) -> bool {
        #[cfg(feature = "kittest")]
        if matches!(self, Self::Transform(_)) {
            return true;
        }
        matches!(
            self,
            Self::Space(_) | Self::Transition(_) | Self::Animation(_)
        )
    }
}

pub(crate) const ATOM_PX: u8 = 0;
pub(crate) const ATOM_FULL: u8 = 1;
pub(crate) const ATOM_FRACTION: u8 = 2;
pub(crate) const ATOM_AUTO: u8 = 3;

pub(crate) const fn decode_atom(kind: u8, value: i16) -> crate::layout::Length {
    match kind {
        ATOM_FULL => crate::layout::Length::Percent(1.0),
        ATOM_FRACTION => crate::layout::Length::Percent(value as f32 / 10_000.0),
        ATOM_AUTO => crate::layout::Length::Auto,
        _ => crate::layout::Length::Px(value as f32),
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
