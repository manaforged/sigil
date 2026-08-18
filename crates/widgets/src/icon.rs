//! Glyph icon — a sized, tone-tinted text glyph.
//!
//! The smallest possible icon primitive: a single Unicode glyph
//! rendered as text, with a semantic [`Tone`] for color and a
//! [`Size`] for the bounding square. Icon-font glyphs (Lucide,
//! Material Symbols, Phosphor, etc.) drop in as the `glyph`
//! argument; the widget itself is font-agnostic.
//!
//! Stroke chrome (nav, titlebar, KPI marks) uses
//! [`crate::widgets::path`] + [`PathNode::fit_size`](crate::widgets::PathNode::fit_size),
//! not this widget. This widget has no vector outline and no emoji font.
//!
//! Accessibility — by default an icon is decorative (no
//! accessible name, screen readers skip it). When the icon is the
//! only content of an interactive control, attach a label so the
//! control gets one:
//!
//! ```ignore
//! use sigil::prelude::*;
//!
//! // Decorative — usually paired with a text sibling.
//! icon("\u{2699}");
//!
//! // Standalone, semantic.
//! icon("\u{2699}").label("Settings").tone(Tone::Primary);
//! ```

use crate::widget_dev::*;
use sigil_kernel::TextContent;
use sigil_style::Tone;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

/// Glyph text-colour atom: step 11 of the current tone's scale.
///
/// Applied to the inner text node so the glyph visually reflects the
/// widget's tone. Without this the text node defaults to
/// `theme.on_surface` (dark gray on light theme, near-invisible on the
/// test harness's black clear-colour background).
static ICON_GLYPH_COLOR: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];

/// Builder returned by [`icon`].
pub struct IconBuilder {
    glyph: TextContent,
    tone: Tone,
    size: Size,
    label: Option<String>,
    style_accumulator: sigil_style::Style,
    layout_overrides: LayoutOverrides,
}

/// Create a glyph icon.
#[must_use]
pub fn icon(glyph: impl Into<TextContent>) -> IconBuilder {
    IconBuilder::new(glyph)
}

impl IconBuilder {
    /// Construct an icon builder defaulting to neutral tone +
    /// `Size::Default` (20 px square).
    #[must_use]
    pub fn new(glyph: impl Into<TextContent>) -> Self {
        Self {
            glyph: glyph.into(),
            tone: Tone::Neutral,
            size: Size::Default,
            label: None,
            style_accumulator: sigil_style::Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Set the semantic tone — drives the foreground color via
    /// [`Tone::scale_role`] step 11.
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Set a semantic theme-resolved size preset.
    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Attach an accessible label — promotes the icon's role to
    /// `Image` and adds the label for screen readers.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        // Icon extents are decoupled from `theme.control_height` —
        // control-height tokens are hit-target sized (24/32/44) and
        // would make `Compact` larger than `Default`. Icon-specific
        // sizes follow a monotonic 16/20/24 ladder.
        let extent = match self.size {
            Size::Compact => 16.0,
            Size::Default => 20.0,
            Size::Wide | Size::Grow => 24.0,
        };
        let glyph_class = match self.size {
            Size::Compact => "text-xs",
            Size::Default => "text-base",
            Size::Wide | Size::Grow => "text-lg",
        };
        let role = self
            .label
            .as_ref()
            .map_or(Role::GenericContainer, |_| Role::Image);
        // The glyph text child is always purely visual — the
        // accessible name lives on the container's `Role::Image +
        // label` for semantic icons, or the icon is decorative and
        // the container's `GenericContainer + no name` hides it. The
        // glyph itself is hidden from hit-test and pointer routing so
        // it does not leak into the a11y tree as a phantom label
        // sibling (adversarial review 2026-05-21).
        let mut container_node = container()
            .role(role)
            .layout(self.layout_overrides.apply(icon_layout(extent)))
            .tone(self.tone)
            .child(
                text(self.glyph)
                    .class(glyph_class)
                    .class(ICON_GLYPH_COLOR)
                    .tone(self.tone)
                    .hittable(false),
            );
        if let Some(label) = self.label {
            container_node = container_node.label(label);
        }
        if !self.style_accumulator.is_empty() {
            container_node = container_node.style(self.style_accumulator);
        }
        container_node
    }
}

fn icon_layout(extent: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(extent, extent)
        .shrink(0.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

impl StyleAccess for IconBuilder {
    fn style_mut(&mut self) -> &mut sigil_style::Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for IconBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for IconBuilder {
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
    fn decorative_icon_uses_generic_container_role() {
        Runtime::scope("icon-decorative-role", || {
            let mut cx = Cx::new_root();
            let id = icon("\u{2699}").build(&mut cx);
            let node = cx.get(id).expect("icon node exists");
            assert_eq!(node.a11y.role, sigil_kernel::Role::GenericContainer);
        });
    }

    #[test]
    fn labeled_icon_promotes_to_image_role() {
        Runtime::scope("icon-labeled-role", || {
            let mut cx = Cx::new_root();
            let id = icon("\u{2699}").label("Settings").build(&mut cx);
            let node = cx.get(id).expect("icon node exists");
            assert_eq!(node.a11y.role, sigil_kernel::Role::Image);
        });
    }

    #[test]
    fn icon_holds_single_text_child() {
        Runtime::scope("icon-one-text-child", || {
            let mut cx = Cx::new_root();
            let id = icon("\u{2699}").build(&mut cx);
            let node = cx.get(id).expect("icon node exists");
            assert_eq!(node.children().len(), 1, "icon wraps one glyph text node");
        });
    }
}
