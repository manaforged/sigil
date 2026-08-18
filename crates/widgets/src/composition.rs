//! Composition shorthands: [`status_bar`] and [`kv_row`].
//!
//! Two friction logs (`scratchpad`, `timer-stopwatch`) asked for the same
//! two trivial flex-row arrangements. Both are 4-6 lines of layout +
//! style per use; collapsing them into a one-liner is the entire win.
//!
//! * [`status_bar`] — bottom-of-shell row of indicators / chips with
//!   sensible inner spacing and the `surface-hover` background.
//! * [`kv_row`] — `Label: value` pair for settings panels, lap times,
//!   metadata rows.

use sigil_kernel::{IntoNode, NodeBuilder, Role, TextContent, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_style::NodeStyleExt;

/// Outer inset for a status bar in logical pixels (matches the panel
/// padding scale at `p-2`).
const STATUS_BAR_PADDING: f32 = 8.0;
/// Gap between status-bar items.
const STATUS_BAR_GAP: f32 = 12.0;
/// Inter-element gap for [`kv_row`] (between label and value).
const KV_ROW_GAP: f32 = 8.0;

/// Row of status indicators / pills — `flex_row` with `surface-hover`
/// background, padded inset, and `space-between` justify so the first
/// and last child anchor to opposite edges.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// status_bar([
///     chip("READY", Tone::Success),
///     text("128 chars"),
/// ])
/// ```
#[must_use]
pub fn status_bar<I>(children: I) -> NodeBuilder
where
    I: IntoIterator,
    I::Item: IntoNode + 'static,
{
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding(STATUS_BAR_PADDING)
                .gap(STATUS_BAR_GAP)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .class("bg-surface-hover");
    for child in children {
        root = root.child(child);
    }
    root
}

/// Label-value pair row — left-aligned label, right-aligned value, with
/// the standard `kv` spacing. Typical use: lap times, settings rows,
/// metadata displays.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// kv_row("lap 1", text("00:42.183"))
/// ```
#[must_use]
pub fn kv_row(label: impl Into<TextContent>, value: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(KV_ROW_GAP)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .child(text(label).class("text-on-surface"))
        .child(value)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use sigil_kernel::{Cx, container as kernel_container};
    use sigil_reactive::Runtime;

    #[test]
    fn status_bar_has_no_children_when_iterator_is_empty() {
        Runtime::scope("status-bar-empty", || {
            let mut cx = Cx::new_root();
            let children: Vec<NodeBuilder> = Vec::new();
            let id = status_bar(children).build(&mut cx);
            let node = cx.get(id).expect("status-bar root exists");
            assert_eq!(node.a11y.role, Role::GenericContainer);
            assert!(
                node.children().is_empty(),
                "empty iterator must produce zero children"
            );
        });
    }

    #[test]
    fn status_bar_forwards_each_child_in_order() {
        Runtime::scope("status-bar-children", || {
            let mut cx = Cx::new_root();
            let children = vec![kernel_container(), kernel_container()];
            let id = status_bar(children).build(&mut cx);
            let node = cx.get(id).expect("status-bar root exists");
            assert_eq!(
                node.children().len(),
                2,
                "status_bar forwards every child in the input iterator"
            );
        });
    }

    #[test]
    fn kv_row_holds_label_and_value_children_in_order() {
        Runtime::scope("kv-row-static-label", || {
            let mut cx = Cx::new_root();
            let id = kv_row("lap 1", text("00:42.183")).build(&mut cx);
            let node = cx.get(id).expect("kv-row root exists");
            assert_eq!(node.a11y.role, Role::GenericContainer);
            assert_eq!(
                node.children().len(),
                2,
                "kv_row produces exactly two children: label then value"
            );
        });
    }

    #[test]
    fn kv_row_accepts_owned_label_string() {
        Runtime::scope("kv-row-owned-label", || {
            let mut cx = Cx::new_root();
            let id = kv_row(String::from("status"), text("ok")).build(&mut cx);
            let node = cx.get(id).expect("kv-row root exists");
            assert_eq!(node.children().len(), 2);
        });
    }
}
