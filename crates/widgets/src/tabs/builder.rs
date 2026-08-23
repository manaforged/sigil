//! Tabs builder.
//!
//! Public surface: [`tabs`] + [`TabsBuilder`]. The builder collects per-tab
//! state (labels, disabled set, optional per-index content), the global
//! reorder + close opt-ins, then materialises a [`Role::TabList`] root
//! container or a `flex_column` wrapper when content routing is in use.

use std::rc::Rc;

use sigil_animation_runtime::presets;
use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{
    ClassSource, DisplayKind, NodeStyleExt, StateHost, StateStyle, Style, StyleAccess, StyleAtom,
    StyleOverlay, StyleVariant, Theme, TransitionId,
};

use crate::interaction::InteractionState;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{Activate, BuilderMutator, activate_key, activate_pointer};
use super::super::transition_seam::install_transition_animators;
use super::drag::{DragState, install_label_drag_hooks};

const TABS_TRANSITION: TransitionId = presets::SMOOTH;
/// Component id for tabs container — per-tab ids OR-in the tab index.
const TABS_COMPONENT_ID: u64 = 0x7AB5_0000;
pub(super) const TAB_HEIGHT: f32 = 30.0;
const TAB_PADDING_X: f32 = 12.0;
const CLOSE_AFFORDANCE_WIDTH: f32 = 16.0;
/// Underline / pill tray height (Tailwind `h-9`).
const STRIP_HEIGHT: f32 = 36.0;

#[derive(Clone, Copy, Default)]
enum StripChrome {
    #[default]
    Bare,
    Line,
    Pill,
}

// ---- atoms ---------------------------------------------------------------

static TAB_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::Transition(TABS_TRANSITION),
];
static TAB_REST: &[StyleAtom] = &[];
static TAB_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static TAB_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static TAB_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed")];
static TAB_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];

// Active-tab atoms: the active tab paints `surface-pressed` at rest so
// the selected affordance reads clearly against neighbouring `surface`
// tabs. Hover / focus / down slots layer on top of the active rest.
static TAB_ACTIVE_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-pressed"),
    StyleAtom::Transition(TABS_TRANSITION),
];
static TAB_ACTIVE_REST: &[StyleAtom] = &[];
static TAB_ACTIVE_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];
static TAB_ACTIVE_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];
static TAB_ACTIVE_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

static TAB_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(TAB_REST, TAB_FOCUS, TAB_HOVER, TAB_DOWN, TAB_DISABLED),
);

static TAB_ACTIVE_VARIANT: StyleVariant = StyleVariant::new(
    TAB_ACTIVE_BASE,
    StateStyle::new(
        TAB_ACTIVE_REST,
        TAB_ACTIVE_FOCUS,
        TAB_ACTIVE_HOVER,
        TAB_ACTIVE_DOWN,
        TAB_DISABLED,
    ),
);

// ---- builder -------------------------------------------------------------

pub(super) type CloseHandler = Rc<dyn Fn(usize) + 'static>;

/// Shared body-factory thunk: invoked from inside the reactive subtree
/// to produce a fresh `IntoNode` for the active tab body.
pub(super) type ContentFactory = Rc<dyn Fn() -> Box<dyn IntoNode>>;

/// One tab's content. `Static` bodies are captured at build time and
/// shown via the existing `Display::None` collapse on inactive tabs.
/// `Factory` bodies are rebuilt every time the active tab lands on them
/// (or any signal read inside the factory mutates), via a single
/// `children_reactive` subtree wired into the content area.
pub(super) enum ContentSlot {
    Static(Box<dyn IntoNode>),
    Factory(ContentFactory),
}

/// Builder returned by [`tabs`].
pub struct TabsBuilder {
    active: Signal<usize>,
    labels: Vec<String>,
    disabled: Vec<usize>,
    aria_label: String,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    contents: Vec<Option<ContentSlot>>,
    reorderable: bool,
    order: Option<Signal<Vec<usize>>>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    list_justify: JustifyContent,
    chrome: StripChrome,
    layout_overrides: LayoutOverrides,
}

/// Create a tabs widget.
///
/// `active` holds the index of the currently selected tab. `labels` is the
/// list of tab captions shown left-to-right.
#[must_use]
pub fn tabs(active: Signal<usize>, labels: Vec<String>) -> TabsBuilder {
    TabsBuilder::new(active, labels)
}

impl TabsBuilder {
    /// Create a tabs builder.
    #[must_use]
    pub fn new(active: Signal<usize>, labels: Vec<String>) -> Self {
        let count = labels.len();
        let mut contents: Vec<Option<ContentSlot>> = Vec::with_capacity(count);
        for _ in 0..count {
            contents.push(None);
        }
        Self {
            active,
            labels,
            disabled: Vec::new(),
            aria_label: String::new(),
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            contents,
            reorderable: false,
            order: None,
            closeable: false,
            close_handler: None,
            list_justify: JustifyContent::Start,
            chrome: StripChrome::Bare,
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Full-width underline bar, chips centered.
    #[must_use]
    pub const fn line(mut self) -> Self {
        self.chrome = StripChrome::Line;
        self
    }

    /// Centered pill tray around the chips.
    #[must_use]
    pub const fn pill(mut self) -> Self {
        self.chrome = StripChrome::Pill;
        self
    }

    /// Pack the tab chips along the strip (`Start` default, `Center`
    /// for a settings-style centered pill list).
    #[must_use]
    pub const fn justify_list(mut self, justify: JustifyContent) -> Self {
        self.list_justify = justify;
        self
    }

    /// Set the accessible label for the tab list container.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = label.into();
        self
    }

    /// Mark the listed tab indices as disabled (non-interactive).
    #[must_use]
    pub fn disabled(mut self, indices: &[usize]) -> Self {
        self.disabled = indices.to_vec();
        self
    }

    /// Add utility classes or atoms after the built-in tabs defaults.
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Add struct-built style atoms after the built-in tabs defaults.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    /// Attach a scoped theme to the tabs root.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    /// Attach a content node to the tab at `index`.
    ///
    /// Only the active tab's content is rendered — inactive entries keep
    /// their kernel node but paint with `DisplayKind::None`. Out-of-range
    /// indices are silently ignored to keep callers from panicking when
    /// the label list shrinks.
    ///
    /// For bodies that need to rebuild reactively (e.g. when a signal
    /// read inside the body mutates), see [`Self::content_for_fn`].
    #[must_use]
    pub fn content_for(mut self, index: usize, node: impl IntoNode + 'static) -> Self {
        if let Some(slot) = self.contents.get_mut(index) {
            *slot = Some(ContentSlot::Static(Box::new(node)));
        }
        self
    }

    /// Attach a factory closure that produces the tab body on demand.
    ///
    /// The closure is invoked whenever (a) the active-tab signal flips
    /// onto `index`, or (b) any signal read inside the closure body
    /// mutates while `index` is the active tab. The body is mounted via
    /// a single `children_reactive` subtree shared across all factory
    /// tabs — the prior body's reactive owners are disposed before the
    /// new body is built (B1 positional identity).
    ///
    /// Out-of-range indices are silently ignored.
    #[must_use]
    pub fn content_for_fn<F, N>(mut self, index: usize, factory: F) -> Self
    where
        F: Fn() -> N + 'static,
        N: IntoNode + 'static,
    {
        if let Some(slot) = self.contents.get_mut(index) {
            let wrapped: ContentFactory = Rc::new(move || Box::new(factory()) as Box<dyn IntoNode>);
            *slot = Some(ContentSlot::Factory(wrapped));
        }
        self
    }

    /// Enable drag-to-reorder on tab labels.
    #[must_use]
    pub const fn reorderable(mut self, enabled: bool) -> Self {
        self.reorderable = enabled;
        self
    }

    /// Bind an observable order signal — host reads to learn the current
    /// tab permutation. When `reorderable(true)` is set, drag updates write
    /// into this signal.
    #[must_use]
    pub const fn order(mut self, signal: Signal<Vec<usize>>) -> Self {
        self.order = Some(signal);
        self
    }

    /// Show a per-tab close affordance.
    #[must_use]
    pub const fn closeable(mut self, enabled: bool) -> Self {
        self.closeable = enabled;
        self
    }

    /// Run `handler` with the original tab index when its close affordance
    /// is activated.
    #[must_use]
    pub fn close(mut self, handler: impl Fn(usize) + 'static) -> Self {
        self.close_handler = Some(Rc::new(handler));
        self
    }

    /// Wrap this tabs widget in a [`container()`] so the result is a
    /// `NodeBuilder` chainable for `.class()`, `.layout()`, etc.
    ///
    /// Without this, `tabs(...).content_for(...)` returns a
    /// [`TabsBuilder`] which `IntoNode`s but does not expose
    /// container-level methods. Equivalent to `container().child(self)`.
    #[must_use]
    pub fn into_container(self) -> NodeBuilder {
        container().child(self)
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let count = self.labels.len();
        let has_content = self.contents.iter().any(Option::is_some);
        let order_signal = self.order;
        let drag = if self.reorderable {
            Some(DragState::new())
        } else {
            None
        };
        // Drain user-supplied state-slot overlays before the base
        // accumulator attaches — each tab clones the overlay and
        // resolves it against its own interaction slot, so
        // `.hover(|s| s.bg(...))` paints the tab under the pointer.
        let user_overlay = self.style_accumulator.take_state_overlay();
        let tab_list = apply_chrome(self.chrome, {
            let list = build_tab_list(
                self.active,
                self.labels,
                &self.disabled,
                &self.aria_label,
                order_signal,
                drag,
                self.closeable,
                self.close_handler.as_ref().map(Rc::clone),
                user_overlay,
            );
            if matches!(self.list_justify, JustifyContent::Start) {
                list
            } else {
                container()
                    .layout(
                        LayoutStyle::flex_row()
                            .fill_width()
                            .justify_content(self.list_justify),
                    )
                    .child(list)
            }
        });
        let mut root = if has_content {
            wrap_with_content(tab_list, self.active, self.contents, count)
        } else {
            tab_list
        };

        for mutate in self.mutators {
            root = mutate(root);
        }

        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        // Strip-only tabs are a fixed-size flex_row of tab chips. With
        // content routing the root is the strip + content column built by
        // `wrap_with_content` — forcing the strip's row layout onto it
        // collapsed the whole widget to a 34px-tall row with the content
        // beside the strip.
        let base = if has_content {
            LayoutStyle::flex_column().align_items(AlignItems::Stretch)
        } else {
            tab_list_layout()
        };
        root.layout(self.layout_overrides.apply(base))
    }
}

impl StyleAccess for TabsBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for TabsBuilder {}

impl WidgetLayoutExt for TabsBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for TabsBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn accessible_label(label: &str) -> AccessibleName {
    if label.is_empty() {
        AccessibleName::Static(String::from("Tabs"))
    } else {
        AccessibleName::Static(label.to_owned())
    }
}

fn tab_list_layout() -> LayoutStyle {
    // Tabs hug their labels (measured by the layout engine's text
    // measure), so the strip's width is content-driven. A fixed
    // per-tab width clipped longer labels mid-glyph (Fable visual
    // review 2026-08-15: "Appearance" sheared to "Appearanc").
    LayoutStyle::flex_row()
        .height(TAB_HEIGHT)
        .align_items(AlignItems::Center)
}

fn apply_chrome(chrome: StripChrome, strip: NodeBuilder) -> NodeBuilder {
    match chrome {
        StripChrome::Bare => strip,
        StripChrome::Line => wrap_line(strip),
        StripChrome::Pill => wrap_pill(strip),
    }
}

fn wrap_line(strip: NodeBuilder) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(STRIP_HEIGHT)
                .shrink(0.0)
                .padding_x(16.0)
                .justify_content(JustifyContent::Center)
                .align_items(AlignItems::Center),
        )
        .class("border-b bg-surface-0")
        .child(strip)
}

fn wrap_pill(strip: NodeBuilder) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .shrink(0.0)
                .padding_y(24.0)
                .justify_content(JustifyContent::Center),
        )
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .height(STRIP_HEIGHT)
                        .padding(4.0)
                        .align_items(AlignItems::Center),
                )
                .class("rounded-md bg-surface-2")
                .child(strip),
        )
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn build_tab_list(
    active: Signal<usize>,
    labels: Vec<String>,
    disabled: &[usize],
    aria_label: &str,
    order: Option<Signal<Vec<usize>>>,
    drag: Option<DragState>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    user_overlay: StyleOverlay,
) -> NodeBuilder {
    let count = labels.len();
    let disabled = Rc::new(disabled.to_vec());

    let mut root = container()
        .role(Role::TabList)
        .label(accessible_label(aria_label))
        .layout(tab_list_layout());

    let visual_order: Vec<usize> = order.map_or_else(
        || identity_order(count),
        |sig| {
            let current = sig.peek();
            if current.len() == count {
                current
            } else {
                identity_order(count)
            }
        },
    );

    for (visual_index, original) in visual_order.iter().copied().enumerate() {
        let label = labels.get(original).cloned().unwrap_or_default();
        let tab = build_tab(
            original,
            visual_index,
            label,
            active,
            &disabled,
            drag,
            closeable,
            close_handler.as_ref().map(Rc::clone),
            user_overlay.clone(),
        );
        root = root.child(tab);
    }

    if let Some(drag_state) = drag {
        // Per-tab move/up handlers live on each tab (see
        // `install_label_drag_hooks`) because the router routes captured
        // pointer events to the captured node, never to ancestors. Bind
        // the order signal + count here so those handlers can reach
        // them via the shared `DragState`.
        drag_state.bind(order, count);
    }

    root
}

fn identity_order(count: usize) -> Vec<usize> {
    (0..count).collect()
}

fn wrap_with_content(
    tab_list: NodeBuilder,
    active: Signal<usize>,
    contents: Vec<Option<ContentSlot>>,
    count: usize,
) -> NodeBuilder {
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().align_items(AlignItems::Stretch))
        .child(tab_list);

    // Collect factories by index so the single reactive subtree can
    // dispatch on the active tab. Static slots keep their existing
    // `Display::None` panel; factory slots feed the reactive subtree.
    let mut factories: Vec<Option<ContentFactory>> = (0..count).map(|_| None).collect();
    let mut any_factory = false;

    for (index, slot) in contents.into_iter().enumerate() {
        match slot {
            Some(ContentSlot::Static(node)) => {
                root = root.child(content_panel(active, index, node, count));
            }
            Some(ContentSlot::Factory(factory)) => {
                if let Some(slot_mut) = factories.get_mut(index) {
                    *slot_mut = Some(factory);
                    any_factory = true;
                }
            }
            None => {}
        }
    }

    if any_factory {
        root = root.child(reactive_content(active, factories));
    }
    root
}

/// One reactive subtree fans out to whichever factory matches the
/// current active tab. Static tabs return an empty Vec — their static
/// panels render via the parallel `Display::None` path above.
fn reactive_content(active: Signal<usize>, factories: Vec<Option<ContentFactory>>) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().align_items(AlignItems::Stretch))
        .children_reactive(move || {
            let idx = active.get();
            factories
                .get(idx)
                .and_then(|slot| slot.as_ref().map(|factory| factory()))
                .into_iter()
                .collect::<Vec<_>>()
        })
}

fn content_panel(
    active: Signal<usize>,
    index: usize,
    node: Box<dyn IntoNode>,
    _count: usize,
) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column())
        .style_reactive(move || content_display(active, index))
        .child(node)
}

fn content_display(active: Signal<usize>, index: usize) -> &'static [StyleAtom] {
    if active.get() == index {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn build_tab(
    index: usize,
    visual_index: usize,
    label: String,
    active: Signal<usize>,
    disabled: &Rc<Vec<usize>>,
    drag: Option<DragState>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    user_overlay: StyleOverlay,
) -> NodeBuilder {
    let is_disabled = disabled.contains(&index);
    let interaction = InteractionState::new(!is_disabled);
    let component_id = TABS_COMPONENT_ID | (index as u64);
    let selected = derived_eq(move || active.get() == index);

    // Variant is chosen statically per-tab from peek() — `style_reactive`
    // below switches state arms inside the chosen variant. When the
    // user clicks another tab, `active` updates and the next render
    // pass rebuilds with the new active variant; that matches button
    // tone semantics (tone is not reactive on a per-instance basis).
    let variant = if active.peek() == index {
        TAB_ACTIVE_VARIANT
    } else {
        TAB_VARIANT
    };
    let base_atoms: &'static [StyleAtom] = variant.base();
    let state_style: StateStyle = if active.peek() == index {
        tab_active_state_style()
    } else {
        tab_state_style()
    };

    // Drag-aware activate: skip activation when a drag was in flight so a
    // pointer-up that ends a drag does not also flip the active tab.
    let drag_for_activate = drag;
    let activate: Activate = Rc::new(move || {
        if is_disabled {
            return;
        }
        if drag_for_activate.is_some_and(DragState::consume_just_dragged) {
            return;
        }
        active.set(index);
    });
    let pointer_activate = Rc::clone(&activate);
    let key_activate = activate;

    let mut node = container()
        .role(Role::Tab)
        .label(label.clone())
        .checked(selected)
        .layout(tab_layout())
        .class("rounded-sm")
        .class(base_atoms)
        .stateful_style(variant, user_overlay, move || interaction.style_state())
        .pointer_up(move |event| activate_pointer(event, interaction, &pointer_activate))
        .key(Key::Enter, move |event| {
            activate_key(event, interaction, &key_activate);
        });

    if let Some(drag_state) = drag {
        node = install_label_drag_hooks(node, drag_state, visual_index);
    }

    node = node.child(tab_label(label));

    if closeable {
        node = node.child(close_button(index, close_handler.as_ref().map(Rc::clone)));
    }

    node = install_transition_animators(node, base_atoms, state_style, component_id, interaction);
    interaction.install(node)
}

fn tab_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .height(TAB_HEIGHT)
        .padding_x(TAB_PADDING_X)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn tab_label(label: String) -> impl IntoNode {
    // Hug width: the text leaf takes its measured width; the tab's
    // padding supplies the chrome. Labels are not hittable on their
    // own — the router captures pointer events to `path.last()`, which
    // would land on this text node and starve the tab's drag handlers.
    // Letting the tab itself be the deepest hit keeps the capture
    // target where the drag handlers live.
    text(label)
        .layout(LayoutStyle::block())
        .class("text-on-surface")
        .hittable(false)
}

fn close_button(index: usize, handler: Option<CloseHandler>) -> impl IntoNode {
    let interaction = InteractionState::new(true);
    let activate: Activate = Rc::new(move || {
        if let Some(cb) = handler.as_ref() {
            cb(index);
        }
    });
    let pointer_activate = Rc::clone(&activate);
    // `InteractionState::install` wires focusable + hittable + the
    // hover/press signals (via pointer_enter / pointer_down) that
    // `release_pointer` consults. Without it `hovered` stays `false`
    // and the close callback never fires under real pointer flow.
    // Install BEFORE the user-supplied pointer hooks below so the
    // install-side pointer_down runs first; the user pointer_down's
    // `stop_propagation` would otherwise skip every later same-node
    // handler in the same `invoke_pointer` loop.
    let node = interaction.install(
        container()
            .role(Role::Button)
            .label(AccessibleName::Static(String::from("Close tab")))
            .layout(LayoutStyle::block().fixed_size(CLOSE_AFFORDANCE_WIDTH, CLOSE_AFFORDANCE_WIDTH))
            .class("rounded-sm")
            .child(close_glyph()),
    );
    node.pointer_down(move |event| {
        // Stop the close pointer-down from bubbling into the
        // surrounding tab; otherwise the tab activator captures the
        // gesture and the close action never fires on release.
        event.stop_propagation();
    })
    .pointer_up(move |event| {
        event.stop_propagation();
        activate_pointer(event, interaction, &pointer_activate);
    })
}

fn close_glyph() -> impl IntoNode {
    text("\u{00D7}")
        .layout(LayoutStyle::block().width(CLOSE_AFFORDANCE_WIDTH))
        .class("text-on-surface")
}

const fn tab_state_style() -> StateStyle {
    StateStyle::new(TAB_REST, TAB_FOCUS, TAB_HOVER, TAB_DOWN, TAB_DISABLED)
}

const fn tab_active_state_style() -> StateStyle {
    StateStyle::new(
        TAB_ACTIVE_REST,
        TAB_ACTIVE_FOCUS,
        TAB_ACTIVE_HOVER,
        TAB_ACTIVE_DOWN,
        TAB_DISABLED,
    )
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
