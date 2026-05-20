//! Modal widget — full-viewport dimmed backdrop with a centered
//! dialog frame whose visibility is gated by a reactive `open` signal.

use sigil_animation_runtime::presets;
use sigil_kernel::{Cx, FocusScopeKind, IntoNode, NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::Signal;
use sigil_style::{
    ColorAtom, DisplayKind, NodeStyleExt, Rgba8, StateHost, Style, StyleAccess, StyleAtom,
    TransitionId,
};

use super::super::shared::BuilderMutator;

const MODAL_TRANSITION: TransitionId = presets::SMOOTH;

/// Default dialog frame width in logical pixels.
const DIALOG_WIDTH: f32 = 320.0;
/// Default dialog frame height in logical pixels.
const DIALOG_HEIGHT: f32 = 180.0;
/// Inner padding around the dialog frame body.
const DIALOG_PADDING: f32 = 16.0;
/// Title bar height in logical pixels.
const TITLE_HEIGHT: f32 = 24.0;
/// Backdrop dimensions — large enough to fully cover any practical
/// viewport. The kernel clamps overflowing absolute children to the
/// parent rect during paint; oversizing here just guarantees the
/// backdrop reaches every edge without needing reactive viewport input.
const BACKDROP_SIZE: f32 = 4_096.0;
/// Logical viewport the golden harness renders at — used to centre the
/// dialog frame inside the root canvas via absolute insets. Application
/// code can shift the dialog via the style accumulator.
const GOLDEN_VIEWPORT_W: f32 = 400.0;
/// See [`GOLDEN_VIEWPORT_W`].
const GOLDEN_VIEWPORT_H: f32 = 280.0;

// 60 % opacity black scrim — alpha 153 ≈ 0.60 * 255.
static BACKDROP_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundColor(ColorAtom::Rgba8(Rgba8 {
        r: 0,
        g: 0,
        b: 0,
        a: 153,
    })),
    StyleAtom::Transition(MODAL_TRANSITION),
];

static DIALOG_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Transition(MODAL_TRANSITION),
];

/// Builder returned by [`modal`].
pub struct ModalBuilder {
    open: Option<Signal<bool>>,
    title: Option<String>,
    children: Vec<Box<dyn FnOnce(NodeBuilder) -> NodeBuilder>>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

/// Create a modal dialog widget.
///
/// Renders a full-viewport dimmed backdrop and a centered dialog frame.
/// Both layers paint with `Display::None` when the `open` signal is
/// `false`; supply the signal via [`ModalBuilder::open`].
#[must_use]
pub fn modal() -> ModalBuilder {
    ModalBuilder::new()
}

impl ModalBuilder {
    /// Create a modal builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            open: None,
            title: None,
            children: Vec::new(),
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
    }

    /// Set the dialog title (rendered in the dialog's title bar).
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Bind the modal's visibility to a reactive boolean signal.
    ///
    /// When the signal reads `false`, both backdrop and dialog frame
    /// paint with `Display::None` and contribute no layout box.
    #[must_use]
    pub const fn open(mut self, open: Signal<bool>) -> Self {
        self.open = Some(open);
        self
    }

    /// Append a child node to the dialog body.
    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.children
            .push(Box::new(move |node: NodeBuilder| node.child(child)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let open = self.open;
        let title_for_label = self.title.clone().unwrap_or_else(|| String::from("Dialog"));

        let backdrop = build_backdrop(open);
        let dialog = build_dialog(open, self.title, self.children, self.style_accumulator);

        let mut root = container()
            .role(Role::GenericContainer)
            .label(title_for_label)
            .layout(root_layout())
            .style_reactive(move || open_display(open))
            .child(backdrop)
            .child(dialog);

        for mutate in self.mutators {
            root = mutate(root);
        }
        root
    }
}

impl Default for ModalBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleAccess for ModalBuilder {
    fn style(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for ModalBuilder {}

impl IntoNode for ModalBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

// ---- structural helpers --------------------------------------------------

fn root_layout() -> LayoutStyle {
    // The root acts as the absolute-positioning container for the
    // backdrop + dialog. Block layout with full size lets the layout
    // engine resolve the children against the host viewport rect.
    LayoutStyle::block().fixed_size(BACKDROP_SIZE, BACKDROP_SIZE)
}

fn build_backdrop(open: Option<Signal<bool>>) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(0.0)
                .fixed_size(BACKDROP_SIZE, BACKDROP_SIZE),
        )
        .class(BACKDROP_BASE)
        .style_reactive(move || open_display(open))
}

fn build_dialog(
    open: Option<Signal<bool>>,
    title: Option<String>,
    children: Vec<Box<dyn FnOnce(NodeBuilder) -> NodeBuilder>>,
    style_accumulator: Style,
) -> NodeBuilder {
    // Centre the dialog inside the golden viewport. With the backdrop
    // root anchored at the host's top-left and the dialog absolutely
    // positioned, the inset = (viewport - dialog) / 2 places the frame
    // dead centre at 400×280. Different viewports can override the
    // position via the style accumulator.
    let left = (GOLDEN_VIEWPORT_W - DIALOG_WIDTH) / 2.0;
    let top = (GOLDEN_VIEWPORT_H - DIALOG_HEIGHT) / 2.0;
    let title_label = title.clone().unwrap_or_else(|| String::from("Dialog"));

    let mut dialog = container()
        .role(Role::Dialog)
        .label(title_label)
        .focusable(true)
        .mutate(|node| {
            node.a11y.focus_scope = Some(FocusScopeKind::Modal);
        })
        .layout(dialog_layout(top, left))
        .class("rounded-md")
        .class(DIALOG_BASE)
        .style_reactive(move || open_display(open));

    if let Some(title_text) = title {
        dialog = dialog.child(dialog_title(title_text));
    }
    for attach in children {
        dialog = attach(dialog);
    }

    if !style_accumulator.is_empty() {
        dialog = dialog.style(style_accumulator);
    }
    dialog
}

fn dialog_layout(top: f32, left: f32) -> LayoutStyle {
    LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset_top(top)
        .inset_left(left)
        .fixed_size(DIALOG_WIDTH, DIALOG_HEIGHT)
        .padding(DIALOG_PADDING)
        .gap(8.0)
        .align_items(AlignItems::Start)
        .justify_content(JustifyContent::Start)
}

fn dialog_title(title: String) -> impl IntoNode {
    text(title)
        .layout(LayoutStyle::block().height(TITLE_HEIGHT))
        .class("text-on-surface")
}

// ---- open / closed visibility --------------------------------------------

fn open_display(open: Option<Signal<bool>>) -> &'static [StyleAtom] {
    if open.is_none_or(|signal| signal.get()) {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}
