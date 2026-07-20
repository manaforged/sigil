//! Turn-order strip — horizontal row of unit chips for battler HUDs.
//!
//! Reuses the [`chip`](super::chip) widget's tone surface for each
//! entry. The chip at `.active(index)` gets an accent-tone highlight
//! plus a focus-ring-style emphasis ring (the same
//! [`FocusRing`](sigil_style::FocusRing) primitive the `command_menu` cursor
//! row uses), both driven reactively off the active-index signal —
//! moving the signal re-highlights the corresponding chip without
//! rebuilding the strip's child list.

use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{FocusRing, NodeStyleExt, Style, StyleAccess};

use super::chip::{ChipTone, chip};

const STRIP_GAP: f32 = 6.0;

/// Multi-class highlight applied to the active actor's chip — accent
/// background + on-accent text, layered on top of the chip's base
/// neutral tone via `class_when` (last-atom-wins resolution).
const ACTIVE_CLASS: &str = "bg-accent text-on-accent";

/// Builder returned by [`turn_strip`].
pub struct TurnStripBuilder {
    entries: Signal<Vec<String>>,
    active: Option<Signal<usize>>,
    style_accumulator: Style,
}

/// Create a horizontal strip of unit chips in turn order.
///
/// - `entries` — ordered unit labels, tracked reactively; the strip's
///   child list rebuilds when the vector changes.
#[must_use]
pub fn turn_strip(entries: Signal<Vec<String>>) -> TurnStripBuilder {
    TurnStripBuilder::new(entries)
}

impl TurnStripBuilder {
    /// Create a turn-strip builder with no active highlight.
    #[must_use]
    pub fn new(entries: Signal<Vec<String>>) -> Self {
        Self {
            entries,
            active: None,
            style_accumulator: Style::new(),
        }
    }

    /// Highlight the chip at `index` as the current actor.
    #[must_use]
    pub const fn active(mut self, index: Signal<usize>) -> Self {
        self.active = Some(index);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let entries = self.entries;
        let active = self.active;
        let style_accumulator = self.style_accumulator;

        let mut node = container()
            .role(Role::GenericContainer)
            .layout(strip_layout())
            .children_reactive(move || {
                entries
                    .get()
                    .into_iter()
                    .enumerate()
                    .map(|(idx, label)| chip_entry(label, idx, active))
                    .collect::<Vec<_>>()
            });

        if !style_accumulator.is_empty() {
            node = node.style(style_accumulator);
        }
        node
    }
}

impl StyleAccess for TurnStripBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for TurnStripBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn strip_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .gap(STRIP_GAP)
        .align_items(AlignItems::Center)
}

/// Build one chip entry, wired with its own `idx == active` derived
/// signal so the highlight tracks the `.active()` signal without
/// touching the sibling chips.
fn chip_entry(label: String, idx: usize, active: Option<Signal<usize>>) -> NodeBuilder {
    let is_active = derived_eq(move || active.is_some_and(|active| active.get() == idx));
    chip(label, ChipTone::Neutral)
        .into_node_builder()
        .class_when(is_active, ACTIVE_CLASS)
        .focus_ring(FocusRing::new(is_active))
}
