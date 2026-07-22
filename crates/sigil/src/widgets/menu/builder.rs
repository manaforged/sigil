//! Menu widget — vertical stack of menu items in a floating panel
//! whose visibility is bound to a reactive `open` signal.

use sigil_animation_runtime::presets;
use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::{Signal, signal};
use sigil_style::{
    DisplayKind, NodeStyleExt, Size, Style, StyleAccess, StyleAtom, Theme, TransitionId,
};

use super::super::shared::BuilderMutator;

const MENU_TRANSITION: TransitionId = presets::SMOOTH;

/// Menu panel width in logical pixels.
const MENU_WIDTH: f32 = 200.0;
/// Separator stripe thickness in logical pixels.
const SEPARATOR_HEIGHT: f32 = 1.0;
/// Logical viewport the golden harness renders at — used to centre the
/// menu inside the canvas via absolute insets when no anchor is set.
/// Application code that needs trigger-anchored placement should call
/// [`MenuBuilder::anchor`] with the cursor / trigger position instead.
const GOLDEN_VIEWPORT_W: f32 = 240.0;
/// See [`GOLDEN_VIEWPORT_W`].
const GOLDEN_VIEWPORT_H: f32 = 280.0;
/// Wrapper size for anchored menus — matches modal's backdrop sentinel
/// so absolute insets are interpreted relative to (0, 0) of the
/// viewport regardless of the surrounding layout.
const ANCHORED_WRAPPER_SIZE: f32 = 4_096.0;
// Disabled rows paint `surface-pressed-strong` (the same token the
// dropdown uses for its disabled trigger). The background-only approach
// gives ~63 sRGB delta against the resting `surface` slot, well clear
// of the JND; per-text opacity would require an Opacity atom on the
// glyph layer, which the kernel currently applies only to backgrounds.

static MENU_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::Transition(MENU_TRANSITION),
];

static ITEM_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];

static ITEM_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];

static SEPARATOR_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

static DISABLED_DIM: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

/// A single row inside a [`MenuBuilder`].
///
/// Construct with [`MenuItem::new`] and refine with the chained
/// [`MenuItem::shortcut`], [`MenuItem::disabled`], and
/// [`MenuItem::submenu`] builder methods.
#[derive(Clone, Debug)]
pub struct MenuItem {
    /// Primary label rendered on the left of the row.
    pub label: String,
    /// Optional shortcut hint rendered on the right of the row.
    pub shortcut: Option<String>,
    /// When `true`, the row paints dimmed and ignores activation.
    pub disabled: bool,
    /// When `true`, the row renders a trailing `›` indicator.
    pub submenu_indicator: bool,
}

impl MenuItem {
    /// Create a menu item with the given label and no shortcut.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            disabled: false,
            submenu_indicator: false,
        }
    }

    /// Set the right-aligned shortcut hint (for example `"Ctrl+S"`).
    #[must_use]
    pub fn shortcut(mut self, hint: impl Into<String>) -> Self {
        self.shortcut = Some(hint.into());
        self
    }

    /// Mark the row as disabled (dimmed, non-activatable).
    #[must_use]
    pub const fn disabled(mut self, yes: bool) -> Self {
        self.disabled = yes;
        self
    }

    /// Render a trailing `›` submenu indicator on the row.
    #[must_use]
    pub const fn submenu(mut self, yes: bool) -> Self {
        self.submenu_indicator = yes;
        self
    }
}

/// Internal row representation — items and separators share the same
/// vertical-stack slot.
enum Row {
    Item(MenuItem),
    Separator,
}

/// Builder returned by [`menu`].
pub struct MenuBuilder {
    open: Signal<bool>,
    rows: Vec<Row>,
    label: Option<String>,
    anchor: Option<(f32, f32)>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

/// Create a menu widget.
///
/// Renders a vertical stack of items in a floating panel. The
/// `open` signal gates visibility: when it reads `false`, the panel
/// paints `Display::None` and contributes no layout box.
///
/// Position via the [`Styled`](sigil_style::Styled) surface (for
/// example `.style(Style::new().padding(8))`). The default placement
/// centres the panel in the golden viewport.
#[must_use]
pub fn menu(open: Signal<bool>) -> MenuBuilder {
    MenuBuilder::new(open)
}

impl MenuBuilder {
    /// Create a menu builder bound to `open`.
    #[must_use]
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            rows: Vec::new(),
            label: None,
            anchor: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
    }

    /// Set an accessible label on the menu root.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Anchor the menu panel at viewport coordinates `(x, y)`.
    ///
    /// When set, the panel paints with its top-left at the given
    /// position instead of being centred in the golden viewport.
    /// Use this for trigger-positioned popovers (right-click context
    /// menus, button-anchored dropdowns, etc.).
    ///
    /// V1 limitation — the anchor is captured at build time. To
    /// reposition between opens, rebuild the menu (rebuild the
    /// containing component, or conditionally insert the menu into
    /// the tree). Reactive re-anchoring waits on the kernel's
    /// reactive-layout primitive landing.
    #[must_use]
    pub const fn anchor(mut self, x: f32, y: f32) -> Self {
        self.anchor = Some((x, y));
        self
    }

    /// Append a menu item row.
    #[must_use]
    pub fn item(mut self, item: MenuItem) -> Self {
        self.rows.push(Row::Item(item));
        self
    }

    /// Append a 1 px separator stripe between items.
    #[must_use]
    pub fn separator(mut self) -> Self {
        self.rows.push(Row::Separator);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let open = self.open;
        let panel_h = panel_height(&self.rows);
        let (top, left, wrapper_size) = if let Some((x, y)) = self.anchor {
            (y, x, ANCHORED_WRAPPER_SIZE)
        } else {
            let centered_left = (GOLDEN_VIEWPORT_W - MENU_WIDTH).max(0.0) / 2.0;
            let centered_top = (GOLDEN_VIEWPORT_H - panel_h).max(0.0) / 2.0;
            (
                centered_top,
                centered_left,
                GOLDEN_VIEWPORT_W.max(GOLDEN_VIEWPORT_H),
            )
        };
        let menu_label = self.label.unwrap_or_else(|| String::from("Menu"));

        let mut panel = container()
            .role(Role::Menu)
            .label(menu_label)
            .layout(panel_layout(top, left, panel_h))
            .class("rounded-md")
            .class("shadow-lg")
            .class(MENU_BASE)
            .style_reactive(move || open_display(open));

        for row in self.rows {
            panel = match row {
                Row::Item(item) => panel.child(build_item(item)),
                Row::Separator => panel.child(build_separator()),
            };
        }

        if !self.style_accumulator.is_empty() {
            panel = panel.style(self.style_accumulator);
        }

        // Wrap the absolute-positioned panel in a block-layout outer
        // root that anchors the absolute insets to the viewport. Without
        // a non-absolute parent, the kernel ignores `inset_top_px` /
        // `inset_left_px` and the panel snaps to (0, 0). Anchored menus
        // use a 4096-square sentinel (matches modal's backdrop) so
        // insets resolve to viewport coordinates; the unanchored
        // fallback keeps the original golden-viewport box for the
        // existing golden suite.
        //
        // The wrapper itself flips `Display::None` in lockstep with the
        // `open` signal — without that gate the 4096-square anchored
        // wrapper expanded surrounding layout even when the menu was
        // closed (caught by the cross-vendor adversarial-review on
        // 2026-05-21).
        let mut root = container()
            .layout(LayoutStyle::block().fixed_size(wrapper_size, wrapper_size))
            .style_reactive(move || open_display(open))
            .child(panel);

        for mutate in self.mutators {
            root = mutate(root);
        }
        root
    }
}

impl StyleAccess for MenuBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for MenuBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

// ---- structural helpers --------------------------------------------------

fn panel_height(rows: &[Row]) -> f32 {
    let theme = Theme::default();
    let item_height = theme.control_height(Size::Default).px_or_zero();
    rows.iter()
        .map(|row| match row {
            Row::Item(_) => item_height,
            Row::Separator => SEPARATOR_HEIGHT,
        })
        .sum::<f32>()
        .max(item_height)
}

fn panel_layout(top: f32, left: f32, height: f32) -> LayoutStyle {
    LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset_top(top)
        .inset_left(left)
        .fixed_size(MENU_WIDTH, height)
        .align_items(AlignItems::Start)
        .justify_content(JustifyContent::Start)
}

fn build_item(item: MenuItem) -> NodeBuilder {
    let MenuItem {
        label,
        shortcut,
        disabled,
        submenu_indicator,
    } = item;

    let hover = signal(false);
    let mut node = container()
        .role(Role::MenuItem)
        .label(label.clone())
        .layout(item_layout())
        .class(ITEM_BASE)
        .style_reactive(move || row_atoms(hover.get(), disabled))
        .pointer_enter(move |_| {
            if !disabled {
                hover.set(true);
            }
        })
        .pointer_leave(move |_| {
            hover.set(false);
        });

    node = node.child(
        text(label)
            .layout(
                LayoutStyle::block()
                    .width(item_label_width(shortcut.as_deref(), submenu_indicator)),
            )
            .class("text-on-surface"),
    );

    if let Some(hint) = shortcut {
        node = node.child(
            text(hint)
                .layout(LayoutStyle::block().width(56.0))
                .class("text-on-surface"),
        );
    }

    if submenu_indicator {
        node = node.child(submenu_swatch());
    }

    node
}

fn item_layout() -> LayoutStyle {
    let theme = Theme::default();
    let item_height = theme.control_height(Size::Default).px_or_zero();
    let row_padding_x = theme.padding(Size::Default).px_or_zero();
    let row_gap = theme.gap(Size::Default).px_or_zero();
    LayoutStyle::flex_row()
        .fixed_size(MENU_WIDTH, item_height)
        .padding(row_padding_x)
        .gap(row_gap)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
}

fn item_label_width(shortcut: Option<&str>, submenu: bool) -> f32 {
    // Row interior = MENU_WIDTH - 2 * ROW_PADDING_X = 176.
    // Reserve room for shortcut (64) + indicator (20) so the
    // SpaceBetween justification can push the trailing children to the
    // row's right edge. The text node itself uses a `block` layout
    // (single-line flow) per the rendering caveat — text inside a
    // sibling flex child renders, text wrapped in an extra container
    // does not.
    let theme = Theme::default();
    let row_padding_x = theme.padding(Size::Default).px_or_zero();
    let interior = 2.0_f32.mul_add(-row_padding_x, MENU_WIDTH);
    let reserved = if shortcut.is_some() { 64.0 } else { 0.0 } + if submenu { 20.0 } else { 0.0 };
    (interior - reserved).max(60.0)
}

const fn row_atoms(hover: bool, disabled: bool) -> &'static [StyleAtom] {
    if disabled {
        DISABLED_DIM
    } else if hover {
        ITEM_HOVER
    } else {
        ITEM_BASE
    }
}

fn submenu_swatch() -> impl IntoNode {
    // Real U+25B8 right-triangle as a content-sized text leaf, matching
    // the dropdown chevron. The old accent swatch was a placeholder for
    // a glyph the earlier width-clipped path collapsed to zero width;
    // §4.5 min-size now gives single-char leaves their measured width,
    // so the glyph paints (confirmed in the stub-measure golden path).
    text("\u{25B8}")
        .layout(LayoutStyle::block())
        .class("text-xs")
        .class("text-on-surface")
}

fn build_separator() -> NodeBuilder {
    container()
        .role(Role::Splitter)
        .layout(
            LayoutStyle::block()
                .fixed_size(MENU_WIDTH, SEPARATOR_HEIGHT)
                .padding(0.0),
        )
        .class(SEPARATOR_BASE)
}

// ---- open / closed visibility --------------------------------------------

fn open_display(open: Signal<bool>) -> &'static [StyleAtom] {
    if open.get() {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sigil_reactive::{Runtime, signal};

    #[test]
    fn anchor_stores_position_for_into_node_builder() {
        Runtime::scope("menu-anchor-stored", || {
            let m = menu(signal(false))
                .item(MenuItem::new("Copy"))
                .anchor(120.0, 80.0);
            assert_eq!(m.anchor, Some((120.0, 80.0)));
        });
    }

    #[test]
    fn unanchored_menu_keeps_none() {
        Runtime::scope("menu-anchor-default-none", || {
            let m = menu(signal(false)).item(MenuItem::new("Copy"));
            assert!(m.anchor.is_none());
        });
    }
}
