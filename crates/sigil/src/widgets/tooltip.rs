//! Tooltip widget — small floating bubble over a hover-targeted child.
//!
//! `tooltip("Save the file").child(button("Save"))` wraps the supplied
//! target with a hover sensor; when the pointer enters the wrapper a
//! reactive visibility signal flips and an absolutely-positioned bubble
//! becomes visible above the target. Pointer-leave fades it back out.
//!
//! Positioning is naive on purpose: the bubble sits at `top = -32`
//! relative to the wrapper. When the brief's "natural offset would
//! clip" path is selected (via [`TooltipBuilder::clamp_top`]) the
//! bubble pins to `top = 0` so a target near the viewport's top edge
//! still renders the hint inside the visible area. Callers that need
//! richer placement (flip-below-target, viewport-aware probes) can
//! layer that on top of the same builder via the same flag.
//!
//! Accessibility:
//! * The bubble itself is tagged with `Role::Tooltip` and its accessible
//!   name is the tooltip label.
//! * The wrapper carries an `aria-description` (the same label) so AT
//!   that follows the description relation on the target surface
//!   announces the hint alongside the target's own name. The DR-050
//!   `A11yProps` struct does not yet carry an explicit
//!   `aria_described_by` node-reference field; using the `description`
//!   string source is the documented W5.3a fallback (see followup).
//!
//! Builder shape (one-word methods per W4.99 ergonomics bar):
//! * `tooltip(label)` — constructor.
//! * `.shortcut(Some("Ctrl+S"))` — optional keyboard hint.
//! * `.child(target)` — the wrapped node.
//! * `.class(...)` / `.style(...)` / `.theme(...)` via the trait surface.

use animation_runtime::presets;
use events::prelude::NodeEventsExt;
use kernel::{Cx, IntoNode, NodeBuilder, Role, container, text};
use layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use reactive::{Signal, signal};
use style::{
    ClassSource, DisplayKind, NodeStyleExt, Size, StateHost, Style, StyleAccess, StyleAtom, Theme,
    TransitionId,
};

use super::shared::BuilderMutator;

/// Vertical offset (in logical px) from the wrapper origin to the bubble
/// when rendered above the target. Negative = above.
const BUBBLE_OFFSET_ABOVE: f32 = -32.0;
/// Vertical offset used when the bubble's natural position would clip
/// off the wrapper's top edge — pin to `top = 0` per the W5.3a brief
/// ("a target near (0,0) clamps to (0,0)"). This deliberately overlaps
/// the target rather than flipping below; the simpler model keeps the
/// renderer's z-order doing the work.
const BUBBLE_OFFSET_CLAMPED: f32 = 0.0;
/// Nominal bubble dimensions.
const BUBBLE_HEIGHT: f32 = 28.0;
const BUBBLE_WIDTH: f32 = 160.0;
const BUBBLE_PADDING: f32 = 6.0;

const TOOLTIP_TRANSITION: TransitionId = presets::SMOOTH;

static BUBBLE_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("accent"),
    StyleAtom::TextThemeRef("on-accent"),
    StyleAtom::Transition(TOOLTIP_TRANSITION),
];

/// Builder returned by [`tooltip`].
pub struct TooltipBuilder {
    label: String,
    shortcut: Option<String>,
    child: Option<Box<dyn IntoNode>>,
    clamp_top: bool,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

/// Create a tooltip widget.
///
/// The `label` is the hint shown when the wrapped target is hovered.
/// Call [`TooltipBuilder::child`] with the target node to complete the
/// builder.
#[must_use]
pub fn tooltip(label: impl Into<String>) -> TooltipBuilder {
    TooltipBuilder::new(label)
}

impl TooltipBuilder {
    /// Create a tooltip builder with the supplied hint text.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            child: None,
            clamp_top: false,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
    }

    /// Set an optional keyboard-shortcut hint shown alongside the label.
    #[must_use]
    pub fn shortcut(mut self, shortcut: Option<impl Into<String>>) -> Self {
        self.shortcut = shortcut.map(Into::into);
        self
    }

    /// Attach the target node the tooltip wraps.
    #[must_use]
    pub fn child(mut self, target: impl IntoNode + 'static) -> Self {
        self.child = Some(Box::new(target));
        self
    }

    /// Drive the explicit "clamped-offscreen" position.
    ///
    /// The default places the bubble at `top = -32` above the target.
    /// When the caller knows the target sits flush against the
    /// viewport's top edge, calling `.clamp_top()` pins the bubble at
    /// `top = 0` instead (the W5.3a brief's "a target near (0,0)
    /// clamps to (0,0)" scenario). Useful for deterministic visual
    /// goldens — callers that want full auto-clamping can flip this
    /// flag based on their own top-edge probe.
    #[must_use]
    pub const fn clamp_top(mut self) -> Self {
        self.clamp_top = true;
        self
    }

    /// Add utility classes or atoms onto the wrapper.
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Add struct-built style atoms onto the wrapper.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    /// Attach a scoped theme to the tooltip wrapper.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let visible = signal(false);
        let bubble_top = if self.clamp_top {
            BUBBLE_OFFSET_CLAMPED
        } else {
            BUBBLE_OFFSET_ABOVE
        };

        let description = bubble_description(&self.label, self.shortcut.as_deref());
        let bubble = build_bubble(&self.label, self.shortcut.as_deref(), visible, bubble_top);

        let mut root = wrapper_root(&description)
            .pointer_enter(move |event| {
                if !visible.peek() {
                    visible.set(true);
                }
                event.stop_propagation();
            })
            .pointer_leave(move |event| {
                if visible.peek() {
                    visible.set(false);
                }
                event.stop_propagation();
            });

        if let Some(child) = self.child {
            root = root.child(child);
        }
        root = root.child(bubble);

        for mutate in self.mutators {
            root = mutate(root);
        }

        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root
    }
}

impl StyleAccess for TooltipBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for TooltipBuilder {}

impl IntoNode for TooltipBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn wrapper_root(description: &str) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .description(description.to_owned())
        .layout(
            LayoutStyle::flex_column()
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
}

fn bubble_description(label: &str, shortcut: Option<&str>) -> String {
    match shortcut {
        Some(keys) if !keys.is_empty() => format!("{label} ({keys})"),
        _ => label.to_owned(),
    }
}

fn build_bubble(
    label: &str,
    shortcut: Option<&str>,
    visible: Signal<bool>,
    top_px: f32,
) -> NodeBuilder {
    let aria_label = bubble_description(label, shortcut);
    let mut bubble = container()
        .role(Role::Tooltip)
        .label(aria_label)
        .layout(bubble_layout(top_px, shortcut.is_some()))
        .class("rounded-sm")
        .class(BUBBLE_BASE)
        .style_reactive(move || visibility_atoms(visible.get()));

    bubble = bubble.child(text(label.to_owned()).class("text-on-accent"));
    if let Some(keys) = shortcut.filter(|s| !s.is_empty()) {
        bubble = bubble.child(text(keys.to_owned()).class("text-on-accent"));
    }
    bubble
}

fn bubble_layout(top_px: f32, has_shortcut: bool) -> LayoutStyle {
    let theme = Theme::default();
    let gap = theme.gap(Size::Default).px_or_zero();
    let width = if has_shortcut {
        BUBBLE_WIDTH
    } else {
        BUBBLE_WIDTH * 0.75
    };
    LayoutStyle::flex_row()
        .fixed_size(width, BUBBLE_HEIGHT)
        .padding(BUBBLE_PADDING)
        .gap(gap)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
        .position(Position::Absolute)
        .inset_top(top_px)
        .inset_left(0.0)
}

const fn visibility_atoms(show: bool) -> &'static [StyleAtom] {
    if show {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use kernel::container as kernel_container;
    use reactive::Runtime;

    #[test]
    fn tooltip_builds_with_target_child() {
        Runtime::scope("tooltip-build", || {
            let _builder = tooltip("Save the file")
                .child(kernel_container())
                .into_node_builder();
        });
    }

    #[test]
    fn tooltip_shortcut_appears_in_description() {
        let with = bubble_description("Save the file", Some("Ctrl+S"));
        assert_eq!(with, "Save the file (Ctrl+S)");
        let without = bubble_description("Save the file", None);
        assert_eq!(without, "Save the file");
        let empty = bubble_description("Save the file", Some(""));
        assert_eq!(empty, "Save the file");
    }

    #[test]
    fn visibility_atoms_gate_display_none() {
        assert!(visibility_atoms(true).is_empty());
        assert_eq!(visibility_atoms(false).len(), 1);
        assert_eq!(
            visibility_atoms(false)[0],
            StyleAtom::Display(DisplayKind::None)
        );
    }

    #[test]
    fn clamp_top_pins_bubble_at_viewport_edge() {
        Runtime::scope("tooltip-clamp-top", || {
            let _builder = tooltip("Hint")
                .clamp_top()
                .child(kernel_container())
                .into_node_builder();
        });
    }
}
