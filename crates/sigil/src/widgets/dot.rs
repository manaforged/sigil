//! Status indicator dot — small filled circle in a semantic tone.
//!
//! The smallest possible status primitive: a fixed-size square with
//! `rounded-full` and a tone-driven background. Pair with a sibling
//! label to express "online" / "draft" / "error" / etc.
//!
//! ```ignore
//! use sigil::prelude::*;
//!
//! dot(Tone::Success);                    // 8 px filled green dot
//! dot(Tone::Warn).size(Size::Default);   // 12 px filled amber dot
//! ```
//!
//! Accessibility — the dot is decorative on its own. Place a text
//! sibling that names the state ("Online", "Draft", etc.) so screen
//! readers see the meaning, not the dot.

use sigil::widget_dev::*;
use sigil_style::{Px, StyleAtom, Tone};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

/// Builder returned by [`dot`].
pub struct DotBuilder {
    tone: Tone,
    size: Size,
    style_accumulator: sigil_style::Style,
    layout_overrides: LayoutOverrides,
}

/// Create a status indicator dot.
#[must_use]
pub fn dot(tone: Tone) -> DotBuilder {
    DotBuilder::new(tone)
}

impl DotBuilder {
    /// Construct a dot builder for the given tone — defaults to
    /// `Size::Compact` (8 px).
    #[must_use]
    pub fn new(tone: Tone) -> Self {
        Self {
            tone,
            size: Size::Compact,
            style_accumulator: sigil_style::Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Size preset — `Compact` = 8, `Default` = 12, `Wide` = 16 px.
    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let extent = match self.size {
            Size::Compact => 8.0,
            Size::Default => 12.0,
            Size::Wide | Size::Grow => 16.0,
        };
        // Two-layer construction. The outer ring is tone step 11 ("high-
        // contrast text" step in Radix) and defines the boundary against
        // the surface; the inner fill is the vibrant step 9 that carries
        // the hue identity. Step-9 amber/green alone measure < 3:1 on
        // light surfaces (visual review 2026-05-21), failing WCAG
        // 1.4.11 for the dot edge — the ring lifts the boundary contrast
        // without sacrificing the saturated colour that makes a status
        // dot recognisable. Mirrors the radio_group mark structure.
        //
        // Radii are clamped to half-extent so the renderer's rounded
        // SDF receives a valid radius (Px(9_999) "rounded-full" falls
        // outside the dot's bounds and clips the entire fill).
        #[allow(
            clippy::cast_possible_truncation,
            reason = "extent is 8.0..=16.0 → half_radius 4..=8, exact i16."
        )]
        let half_radius = Px((extent / 2.0) as i16);
        let inner_extent = (extent - 2.0_f32).max(2.0);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "inner_extent is 6.0..=14.0 → half 3..=7, exact i16."
        )]
        let inner_radius = Px((inner_extent / 2.0) as i16);
        let inner = container()
            .layout(dot_inner_layout(inner_extent))
            .style(sigil_style::Style::new().radius(inner_radius.0))
            .tone(self.tone)
            .class(DOT_FILL_ATOMS);
        let mut node = container()
            .role(Role::GenericContainer)
            .layout(self.layout_overrides.apply(dot_outer_layout(extent)))
            .style(sigil_style::Style::new().radius(half_radius.0))
            .tone(self.tone)
            .class(DOT_RING_ATOMS)
            .child(inner);
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node
    }
}

/// Inner fill — step 9 vibrant solid (carries the hue identity).
static DOT_FILL_ATOMS: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];

/// Outer ring — step 11 high-contrast tonal (lifts the dot edge to
/// ≥4.5:1 against light surfaces for the low-luminance hues amber /
/// yellow / green where step 9 alone fails WCAG 1.4.11).
static DOT_RING_ATOMS: &[StyleAtom] = &[StyleAtom::BgToneStep(11)];

fn dot_outer_layout(extent: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(extent, extent)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn dot_inner_layout(extent: f32) -> LayoutStyle {
    LayoutStyle::flex_row().fixed_size(extent, extent)
}

impl StyleAccess for DotBuilder {
    fn style_mut(&mut self) -> &mut sigil_style::Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for DotBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for DotBuilder {
    fn build(self, cx: &mut sigil_kernel::Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sigil_kernel::Cx;
    use sigil_reactive::Runtime;

    #[test]
    fn dot_uses_generic_container_role() {
        Runtime::scope("dot-role", || {
            let mut cx = Cx::new_root();
            let id = dot(Tone::Success).build(&mut cx);
            let node = cx.get(id).expect("dot node exists");
            assert_eq!(node.a11y.role, sigil_kernel::Role::GenericContainer);
        });
    }

    #[test]
    fn dot_wraps_step9_fill_in_step11_ring() {
        // Two-layer construction: outer step-11 ring + inner step-9
        // fill. The ring lifts edge contrast against the surface so
        // low-luminance hues (amber / yellow / green) still meet WCAG
        // 1.4.11 at the 8 px Compact size.
        Runtime::scope("dot-ring-and-fill", || {
            let mut cx = Cx::new_root();
            let id = dot(Tone::Warn).build(&mut cx);
            let node = cx.get(id).expect("dot node exists");
            assert_eq!(
                node.children().len(),
                1,
                "dot is outer ring + one inner fill child",
            );
        });
    }
}
