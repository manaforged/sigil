//! Menu widget — vertical stack of menu items in a floating panel
//! whose visibility is bound to a reactive `open` signal.

use animation_runtime::presets;
use events::prelude::NodeEventsExt;
use kernel::{Cx, IntoNode, NodeBuilder, Role, container, text};
use layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use reactive::{Signal, signal};
use style::{
    DisplayKind, NodeStyleExt, Size, StateHost, Style, StyleAccess, StyleAtom, Theme, TransitionId,
};

use super::super::shared::BuilderMutator;

const MENU_TRANSITION: TransitionId = presets::SMOOTH;

/// Menu panel width in logical pixels.
const MENU_WIDTH: f32 = 200.0;
/// Separator stripe thickness in logical pixels.
const SEPARATOR_HEIGHT: f32 = 1.0;
/// Submenu indicator dimensions. The indicator is a small `accent`
/// swatch at the row's trailing edge — the same affordance pattern the
/// dropdown trigger uses for its down-chevron. A textual `›` glyph was
/// tried first but the kernel's text raster path (with the goldens'
/// stub text measure) collapses single-character siblings inside a
/// flex row to zero width, so the swatch is the deterministic
/// "more here" cue.
const SUBMENU_INDICATOR_SIZE: f32 = 8.0;
/// Logical viewport the golden harness renders at — used to centre the
/// menu inside the canvas via absolute insets. Application code can
/// shift the menu via the style accumulator.
const GOLDEN_VIEWPORT_W: f32 = 240.0;
/// See [`GOLDEN_VIEWPORT_W`].
const GOLDEN_VIEWPORT_H: f32 = 280.0;
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
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

/// Create a menu widget.
///
/// Renders a vertical stack of items in a floating panel. The
/// `open` signal gates visibility: when it reads `false`, the panel
/// paints `Display::None` and contributes no layout box.
///
/// Position via the [`Styled`](style::Styled) surface (for
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
        let left = (GOLDEN_VIEWPORT_W - MENU_WIDTH).max(0.0) / 2.0;
        let top = (GOLDEN_VIEWPORT_H - panel_h).max(0.0) / 2.0;
        let menu_label = self.label.unwrap_or_else(|| String::from("Menu"));

        let mut panel = container()
            .role(Role::Menu)
            .label(menu_label)
            .layout(panel_layout(top, left, panel_h))
            .class("rounded-md")
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
        // `inset_left_px` and the panel snaps to (0, 0). Modal uses the
        // same wrapping pattern with a 4096-square backdrop sentinel —
        // here the wrapper is the viewport rect itself.
        let mut root = container()
            .layout(LayoutStyle::block().fixed_size(GOLDEN_VIEWPORT_W, GOLDEN_VIEWPORT_H))
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

impl StateHost for MenuBuilder {}

impl IntoNode for MenuBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
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

static SUBMENU_BG: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

fn submenu_swatch() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(SUBMENU_INDICATOR_SIZE, SUBMENU_INDICATOR_SIZE))
        .class("rounded-sm")
        .class(SUBMENU_BG)
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
