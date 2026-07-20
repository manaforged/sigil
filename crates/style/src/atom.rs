//! Style atom vocabulary.

/// One Tailwind-style spacing step, in logical pixels.
///
/// This matches the common shadcn/Tailwind convention where one
/// spacing unit is 0.25rem and the base rem is 16px.
pub const SPACING_STEP: f32 = 4.0;

/// Logical pixel value stored without heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Px(pub i16);

/// Size value for width and height utilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    /// A fixed logical pixel size.
    Px(Px),
    /// Fills the available parent extent.
    Full,
    /// Lets the layout engine choose the size.
    Auto,
}

/// Concrete 8-bit sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

/// Colour atom used by struct-style APIs and resolved utility classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorAtom {
    /// Concrete colour value.
    Rgba8(Rgba8),
    /// Runtime theme token reference.
    ThemeRef(&'static str),
}

/// Display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayKind {
    /// Flex layout.
    Flex,
    /// Block layout.
    Block,
    /// No layout box.
    None,
}

/// Flex direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlexDir {
    /// Main axis flows left to right.
    Row,
    /// Main axis flows top to bottom.
    Column,
    /// Main axis flows right to left.
    RowReverse,
    /// Main axis flows bottom to top.
    ColumnReverse,
}

/// Cross-axis alignment keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignKind {
    /// Align to the start edge.
    Start,
    /// Align to the centre.
    Center,
    /// Align to the end edge.
    End,
    /// Stretch to fill the cross axis.
    Stretch,
    /// Distribute with space between items.
    Between,
    /// Distribute with space around items.
    Around,
}

/// Main-axis distribution keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JustifyKind {
    /// Align to the start edge.
    Start,
    /// Align to the centre.
    Center,
    /// Align to the end edge.
    End,
    /// Stretch to fill the main axis.
    Stretch,
    /// Distribute with space between items.
    Between,
    /// Distribute with space around items.
    Around,
    /// Distribute with equal space between and around items.
    Evenly,
}

/// Flex wrap keyword — maps to `sigil_layout::FlexWrap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlexWrapKind {
    /// Single line (default).
    NoWrap,
    /// Multi-line: items wrap onto new lines.
    Wrap,
}

/// Overflow clipping behaviour for a single axis.
///
/// Maps to `sigil_layout::Overflow` during style resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverflowKind {
    /// Content extends past the box without clipping.
    Visible,
    /// Content past the box is clipped at the border edge.
    Hidden,
    /// Same as `Hidden` but reserves space for a scrollbar.
    Scroll,
    /// CSS `auto`: currently aliases [`Scroll`](Self::Scroll)
    /// (scrollbars-when-needed — our Scroll semantics). Kept as a
    /// distinct variant so a future auto-specific path has a home.
    Auto,
}

/// Stable transition preset identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransitionId(pub u16);

/// Millisecond duration encoded in 16 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MillisU16(pub u16);

/// Renderer-facing style values after theme resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderStyle {
    /// Resolved background fill.
    pub background_color: Option<Rgba8>,
    /// Resolved text fill.
    pub text_color: Option<Rgba8>,
    /// Border radius in logical pixels.
    pub border_radius: Option<Px>,
    /// Border stroke width in logical pixels.
    pub border_width: Option<Px>,
    /// Resolved border stroke colour. `None` with a set width means
    /// "currentColor": the paint layer falls back to the resolved text
    /// colour, then the theme's on-surface (the CSS border shape).
    pub border_color: Option<Rgba8>,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
}

impl Default for RenderStyle {
    fn default() -> Self {
        Self {
            background_color: None,
            text_color: None,
            border_radius: None,
            border_width: None,
            border_color: None,
            opacity: 10_000,
        }
    }
}

/// Atomic style operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleAtom {
    /// Uniform padding.
    PaddingAll(Px),
    /// Horizontal padding.
    PaddingX(Px),
    /// Vertical padding.
    PaddingY(Px),
    /// Top padding.
    PaddingTop(Px),
    /// Right padding.
    PaddingRight(Px),
    /// Bottom padding.
    PaddingBottom(Px),
    /// Left padding.
    PaddingLeft(Px),
    /// Uniform margin.
    MarginAll(Px),
    /// Horizontal margin.
    MarginX(Px),
    /// Vertical margin.
    MarginY(Px),
    /// Top margin.
    MarginTop(Px),
    /// Right margin.
    MarginRight(Px),
    /// Bottom margin.
    MarginBottom(Px),
    /// Left margin.
    MarginLeft(Px),
    /// Flex or grid gap.
    Gap(Px),
    /// Main-axis (width-axis) gap only — `gap-x-N` in the class grammar.
    GapX(Px),
    /// Cross-axis (height-axis) gap only — `gap-y-N` in the class grammar.
    GapY(Px),
    /// Preferred width.
    Width(Dimension),
    /// Preferred height.
    Height(Dimension),
    /// Minimum width.
    MinWidth(Dimension),
    /// Maximum width.
    MaxWidth(Dimension),
    /// Minimum height.
    MinHeight(Dimension),
    /// Maximum height.
    MaxHeight(Dimension),
    /// Display mode.
    Display(DisplayKind),
    /// Flex direction.
    FlexDirection(FlexDir),
    /// Flex wrap behaviour.
    FlexWrap(FlexWrapKind),
    /// Cross-axis alignment.
    AlignItems(AlignKind),
    /// Main-axis distribution.
    JustifyContent(JustifyKind),
    /// Flex grow encoded as `value * 1000`.
    FlexGrow(u16),
    /// Flex shrink encoded as `value * 1000`.
    FlexShrink(u16),
    /// Concrete or nested theme background colour.
    BackgroundColor(ColorAtom),
    /// Runtime theme-token background colour.
    BackgroundThemeRef(&'static str),
    /// Concrete or nested theme text colour.
    TextColor(ColorAtom),
    /// Runtime theme-token text colour.
    TextThemeRef(&'static str),
    /// Background colour resolved as the current tone's scale at the
    /// given step (1..=12).
    ///
    /// The resolving engine consults the node's
    /// [`ToneContext`](crate::ToneContext) extension to pick which
    /// scale to read; absent context defaults to `Tone::Neutral`.
    /// This is the tone-parameterized counterpart to
    /// [`BackgroundThemeRef`](Self::BackgroundThemeRef) — one atom
    /// covers every tone, instead of a hardcoded `"error-9"` string.
    BgToneStep(u8),
    /// Text colour resolved as the current tone's scale at the given
    /// step. See [`BgToneStep`](Self::BgToneStep) for resolution
    /// semantics.
    FgToneStep(u8),
    /// Text colour resolved as the current tone's scale's WCAG-passing
    /// `on_solid` contrast color — the readable text for painting on
    /// `step(9)`. Use on solid-variant widget labels where
    /// `FgToneStep(12)` would fail contrast (most prominently:
    /// warm/yellow tones in dark mode).
    FgOnSolid,
    /// Border radius.
    BorderRadius(Px),
    /// Border stroke width in logical pixels (all four sides).
    /// Box-model-affecting: the layout solvers consume it as border
    /// edge thickness; the renderer paints it as an inside-aligned
    /// stroke (DR-057 D3).
    BorderWidth(Px),
    /// Concrete border stroke colour.
    BorderColor(ColorAtom),
    /// Theme-token border stroke colour.
    BorderThemeRef(&'static str),
    /// Border colour taken from the active tone scale at `step`.
    BorderToneStep(u8),
    /// Opacity encoded as `value * 10000`.
    Opacity(u16),
    /// Font size.
    FontSize(Px),
    /// Font weight.
    FontWeight(u16),
    /// Font family name (e.g. `"sans"`, `"mono"`, `"serif"`). The
    /// text engine resolves this against its registered font
    /// collection; unknown families fall back to the primary face.
    FontFamily(&'static str),
    /// Line height.
    LineHeight(Px),
    /// Overflow clipping on the inline (width) axis.
    OverflowX(OverflowKind),
    /// Overflow clipping on the block (height) axis.
    OverflowY(OverflowKind),
    /// Deferred transition preset.
    Transition(TransitionId),
    /// Deferred transition duration.
    TransitionDuration(MillisU16),
    /// Unsupported theme-style token kept for diagnostics.
    UnknownThemeRef(&'static str),
}
