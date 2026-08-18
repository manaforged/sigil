//! Modal widget — full-viewport dimmed backdrop with a centered
//! dialog frame whose visibility is gated by a reactive `show` signal.

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use sigil_animation_runtime::presets;
use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{
    Cx, FocusScopeKind, IntoNode, Live, NodeBuilder, Role, container, queue_announcement, text,
};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::{Signal, effect_named, on_cleanup, signal};
use sigil_style::{
    DisplayKind, FocusRing, NodeStyleExt, Style, StyleAccess, StyleAtom, TransitionId,
};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

use super::super::shared::BuilderMutator;

const MODAL_TRANSITION: TransitionId = presets::SMOOTH;

/// Inner padding around the dialog frame body.
const DIALOG_PADDING: f32 = 16.0;
/// Title bar height in logical pixels.
const TITLE_HEIGHT: f32 = 24.0;
/// Backdrop dimensions — large enough to fully cover any practical
/// viewport. The kernel clamps overflowing absolute children to the
/// parent rect during paint; oversizing here just guarantees the
/// backdrop reaches every edge without needing reactive viewport input.
const BACKDROP_SIZE: f32 = 4_096.0;

// Dim scrim — resolves to the theme's "scrim" token (60 % opacity black
// by default; see `sigil_style::theme::Theme::built_in_color`). Themable
// rather than a hardcoded literal so a preset can rebind it.
static BACKDROP_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("scrim"),
    StyleAtom::Transition(MODAL_TRANSITION),
];

static DIALOG_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Transition(MODAL_TRANSITION),
];

/// Where a [`sheet`] docks. [`modal`] stays centered (`None`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    /// Leading edge.
    Left,
    /// Trailing edge. Default for [`sheet`].
    Right,
    /// Bottom edge.
    Bottom,
}

/// Builder returned by [`modal`] and [`sheet`].
pub struct ModalBuilder {
    show: Signal<bool>,
    title: Option<String>,
    children: Vec<Box<dyn IntoNode>>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    trap: bool,
    escape_closes: bool,
    dismiss_on_backdrop: bool,
    side: Option<Side>,
    layout_overrides: LayoutOverrides,
}

/// Centered dialog. Pass the open signal, same as `checkbox(on)`.
#[must_use]
pub fn modal(show: Signal<bool>) -> ModalBuilder {
    ModalBuilder::new(show)
}

/// Panel on an edge. Same builder as [`modal`].
///
/// Defaults: right side, backdrop click dismisses. Call
/// [`.side`](ModalBuilder::side) for left or bottom.
#[must_use]
pub fn sheet(show: Signal<bool>) -> ModalBuilder {
    ModalBuilder::new(show)
        .side(Side::Right)
        .dismiss_on_backdrop(true)
}

impl ModalBuilder {
    /// Create a modal builder bound to `show`.
    #[must_use]
    pub fn new(show: Signal<bool>) -> Self {
        Self {
            show,
            title: None,
            children: Vec::new(),
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            trap: true,
            escape_closes: true,
            dismiss_on_backdrop: false,
            side: None,
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Dock the frame to an edge. `None` (default for [`modal`])
    /// centers it.
    #[must_use]
    pub const fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
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
    pub const fn show(mut self, show: Signal<bool>) -> Self {
        self.show = show;
        self
    }

    /// Enable or disable the modal focus trap marker.
    ///
    /// Enabled by default. Passing `false` opts out of registering the
    /// dialog as a modal focus scope.
    #[must_use]
    pub const fn trap(mut self, enabled: bool) -> Self {
        self.trap = enabled;
        self
    }

    /// Enable or disable Escape-key dismissal.
    ///
    /// Enabled by default. When enabled, Escape closes the topmost open
    /// modal by setting its `show` signal to `false`.
    #[must_use]
    pub const fn escape_closes(mut self, enabled: bool) -> Self {
        self.escape_closes = enabled;
        self
    }

    /// Enable or disable backdrop-click dismissal.
    ///
    /// Disabled by default. When enabled, pointer-down on the backdrop
    /// closes the modal by setting its `show` signal to `false`.
    #[must_use]
    pub const fn dismiss_on_backdrop(mut self, enabled: bool) -> Self {
        self.dismiss_on_backdrop = enabled;
        self
    }

    /// Append a child node to the dialog body.
    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let show = self.show;
        let title_for_label = self.title.clone().unwrap_or_else(|| String::from("Dialog"));

        install_open_announcement(show, &title_for_label);
        if self.escape_closes {
            register_escape_modal(show);
        }

        let backdrop = build_backdrop(show, self.dismiss_on_backdrop);
        let dialog = build_dialog(DialogSpec {
            show,
            title: self.title,
            children: self.children,
            style_accumulator: self.style_accumulator,
            trap: self.trap,
            side: self.side,
            layout_overrides: self.layout_overrides,
        });

        let mut root = container()
            .role(Role::GenericContainer)
            .label(title_for_label)
            .layout(root_layout(self.side))
            .style_reactive(move || show_display(show))
            .child(backdrop)
            .child(dialog);
        if self.escape_closes {
            root = root.key(Key::Escape, |ev| {
                if close_top_escape_modal() {
                    ev.stop_propagation();
                }
            });
        }

        for mutate in self.mutators {
            root = mutate(root);
        }

        root
    }
}

impl WidgetLayoutExt for ModalBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

/// Register a reactive effect that announces the dialog title to
/// assistive tech on every `false → true` rising edge of `show`.
///
/// Initial state — including a modal that opens with `show == true`
/// at first read — is treated as steady, not a rising edge: ARIA-style
/// live regions should fire when the modal *opens*, not when the host
/// mounts with the dialog already visible. Re-opens (`false → true`
/// after a `true → false`) do re-announce.
///
/// The effect does not need `&mut Cx`. It queues the announcement via
/// [`queue_announcement`]; the host drains it through
/// [`sigil_kernel::Cx::flush_announcements`] on the next frame tick.
fn install_open_announcement(show: Signal<bool>, title: &str) {
    let prev = Rc::new(Cell::new(true));
    let title = title.to_owned();
    effect_named("sigil::modal::open_announce", move || {
        let open_now = show.get();
        let was_open = prev.replace(open_now);
        // Rising edge: only fire when we just transitioned closed -> open.
        // `was_open == true` covers both steady-open and the implicit
        // initial read (prev seeded to true), suppressing the mount-time
        // announce.
        if open_now && !was_open {
            queue_announcement(title.clone(), Live::Polite);
        }
    });
}

#[derive(Clone, Copy)]
struct EscapeModal {
    id: u64,
    show: Signal<bool>,
}

thread_local! {
    static ESCAPE_MODALS: RefCell<Vec<EscapeModal>> = const {
        RefCell::new(Vec::new())
    };
}

static NEXT_ESCAPE_MODAL_ID: AtomicU64 = AtomicU64::new(1);

fn register_escape_modal(show: Signal<bool>) {
    let id = NEXT_ESCAPE_MODAL_ID.fetch_add(1, Ordering::Relaxed);
    ESCAPE_MODALS.with(|stack| stack.borrow_mut().push(EscapeModal { id, show }));
    on_cleanup(move || {
        ESCAPE_MODALS.with(|stack| {
            stack.borrow_mut().retain(|modal| modal.id != id);
        });
    });
}

fn close_top_escape_modal() -> bool {
    ESCAPE_MODALS.with(|stack| {
        let top = stack
            .borrow()
            .iter()
            .rev()
            .copied()
            .find(|modal| modal.show.peek());
        top.is_some_and(|modal| {
            modal.show.set(false);
            true
        })
    })
}

impl StyleAccess for ModalBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for ModalBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

// ---- structural helpers --------------------------------------------------

fn root_layout(side: Option<Side>) -> LayoutStyle {
    let root = match side {
        None | Some(Side::Bottom) => LayoutStyle::flex_column(),
        Some(Side::Left | Side::Right) => LayoutStyle::flex_row(),
    };
    let (align, justify) = match side {
        None => (AlignItems::Center, JustifyContent::Center),
        Some(Side::Left) => (AlignItems::Stretch, JustifyContent::Start),
        Some(Side::Right | Side::Bottom) => (AlignItems::Stretch, JustifyContent::End),
    };
    root.fill().align_items(align).justify_content(justify)
}

fn build_backdrop(show: Signal<bool>, dismiss_on_backdrop: bool) -> NodeBuilder {
    let mut backdrop = container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(0.0)
                .fixed_size(BACKDROP_SIZE, BACKDROP_SIZE),
        )
        .class(BACKDROP_BASE)
        .style_reactive(move || show_display(show));
    if dismiss_on_backdrop {
        backdrop = backdrop.pointer_down(move |ev| {
            if show.peek() {
                show.set(false);
                ev.stop_propagation();
            }
        });
    }
    backdrop
}

/// Dialog frame inputs, moved out of [`ModalBuilder`].
struct DialogSpec {
    show: Signal<bool>,
    title: Option<String>,
    children: Vec<Box<dyn IntoNode>>,
    style_accumulator: Style,
    trap: bool,
    side: Option<Side>,
    layout_overrides: LayoutOverrides,
}

fn build_dialog(spec: DialogSpec) -> NodeBuilder {
    let DialogSpec {
        show,
        title,
        children,
        style_accumulator,
        trap,
        side,
        layout_overrides,
    } = spec;
    let title_label = title.clone().unwrap_or_else(|| String::from("Dialog"));
    // Focus-visible ring on the dialog frame itself — the modal's own
    // focusable primary action (WAI-ARIA dialog pattern: the dialog is
    // the tab-ring entry point). Mirrors `InteractionState`'s
    // focus_in/focus_out → ring_visible wiring in `interaction.rs`,
    // scoped down to focus-only (no hover/press) since only Role::Dialog
    // itself carries `.focusable(true)` here.
    let ring_visible = signal(false);

    let mut dialog = container()
        .role(Role::Dialog)
        .label(title_label)
        .focusable(true)
        .layout(layout_overrides.apply(dialog_layout(side)))
        .class("rounded-md")
        .class("shadow-xl")
        .class(DIALOG_BASE)
        .style_reactive(move || show_display(show))
        .focus_ring(FocusRing::new(ring_visible))
        .focus_in(move || ring_visible.set(sigil_events::focus_from_keyboard()))
        .focus_out(move || ring_visible.set(false));
    if trap {
        dialog = dialog.mutate(|node| {
            node.a11y.focus_scope = Some(FocusScopeKind::Modal);
        });
    }

    if let Some(title_text) = title {
        dialog = dialog.child(dialog_title(title_text));
    }
    for child in children {
        dialog = dialog.child(child);
    }

    if !style_accumulator.is_empty() {
        dialog = dialog.style(style_accumulator);
    }
    dialog
}

const SHEET_WIDTH: f32 = 360.0;
const SHEET_HEIGHT: f32 = 320.0;

fn dialog_layout(side: Option<Side>) -> LayoutStyle {
    let base = LayoutStyle::flex_column()
        .padding(DIALOG_PADDING)
        .gap(8.0)
        .align_items(AlignItems::Start)
        .justify_content(JustifyContent::Start)
        .shrink(0.0);
    match side {
        None => base,
        Some(Side::Left | Side::Right) => base.fill_height().width(SHEET_WIDTH),
        Some(Side::Bottom) => base.fill_width().height(SHEET_HEIGHT),
    }
}

fn dialog_title(title: String) -> impl IntoNode {
    text(title)
        .layout(LayoutStyle::block().height(TITLE_HEIGHT))
        .class("text-on-surface")
}

// ---- show / closed visibility --------------------------------------------

fn show_display(show: Signal<bool>) -> &'static [StyleAtom] {
    if show.get() {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sigil_reactive::Runtime;

    #[test]
    fn sheet_docks_a_dialog_on_the_right() {
        Runtime::scope("sheet-right", || {
            let mut cx = Cx::new_root();
            let show = signal(true);
            let root = cx.add(sheet(show).title("Filters"));
            let frame = cx
                .get(root)
                .expect("root")
                .children()
                .iter()
                .copied()
                .find(|id| matches!(cx.get(*id).expect("child").a11y.role, Role::Dialog))
                .expect("dialog");
            let style = cx
                .get(frame)
                .expect("frame")
                .get_extension::<LayoutStyle>()
                .expect("layout")
                .clone();
            assert_eq!(
                style.data().size.width,
                sigil_layout::prelude::Dimension::Px(360.0)
            );
            assert_eq!(
                style.data().size.height,
                sigil_layout::prelude::Dimension::Fill
            );
        });
    }
}
