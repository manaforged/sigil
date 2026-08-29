use std::rc::Rc;

use sigil_animation_runtime::presets;
use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{
    ClassSource, NodeStyleExt, StateHost, StateStyle, Style, StyleAccess, StyleAtom, StyleOverlay,
    StyleVariant, Theme, TransitionId,
};

use crate::interaction::InteractionState;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{Activate, BuilderMutator, activate_key, activate_pointer};
use super::super::transition_seam::install_transition_animators;
use super::drag::{DragState, install_label_drag_hooks};

const TABS_TRANSITION: TransitionId = presets::SMOOTH;
const TABS_COMPONENT_ID: u64 = 0x7AB5_0000;
pub(super) const TAB_HEIGHT: f32 = 30.0;
const TAB_PADDING_X: f32 = 12.0;
const CLOSE_AFFORDANCE_WIDTH: f32 = 16.0;
const STRIP_HEIGHT: f32 = 36.0;

#[derive(Clone, Copy, Default)]
enum StripChrome {
    #[default]
    Bare,
    Line,
    Pill,
}

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

pub(super) type CloseHandler = Rc<dyn Fn(usize) + 'static>;

pub(super) type ContentFactory = Rc<dyn Fn() -> Box<dyn IntoNode>>;

pub(super) enum ContentSlot {
    Static(Box<dyn IntoNode>),
    Factory(ContentFactory),
}

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

#[must_use]
pub fn tabs(active: Signal<usize>, labels: Vec<String>) -> TabsBuilder {
    TabsBuilder::new(active, labels)
}

impl TabsBuilder {
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

    #[must_use]
    pub const fn line(mut self) -> Self {
        self.chrome = StripChrome::Line;
        self
    }

    #[must_use]
    pub const fn pill(mut self) -> Self {
        self.chrome = StripChrome::Pill;
        self
    }

    #[must_use]
    pub const fn justify_list(mut self, justify: JustifyContent) -> Self {
        self.list_justify = justify;
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = label.into();
        self
    }

    #[must_use]
    pub fn disabled(mut self, indices: &[usize]) -> Self {
        self.disabled = indices.to_vec();
        self
    }

    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    #[must_use]
    pub fn content_for(mut self, index: usize, node: impl IntoNode + 'static) -> Self {
        if let Some(slot) = self.contents.get_mut(index) {
            *slot = Some(ContentSlot::Static(Box::new(node)));
        }
        self
    }

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

    #[must_use]
    pub const fn reorderable(mut self, enabled: bool) -> Self {
        self.reorderable = enabled;
        self
    }

    #[must_use]
    pub const fn order(mut self, signal: Signal<Vec<usize>>) -> Self {
        self.order = Some(signal);
        self
    }

    #[must_use]
    pub const fn closeable(mut self, enabled: bool) -> Self {
        self.closeable = enabled;
        self
    }

    #[must_use]
    pub fn close(mut self, handler: impl Fn(usize) + 'static) -> Self {
        self.close_handler = Some(Rc::new(handler));
        self
    }

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

#[allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "the argument list mirrors the upstream API contract"
)]
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
        drag_state.bind(order, count);
    }

    root
}

fn identity_order(count: usize) -> Vec<usize> {
    (0..count).collect()
}

fn tab_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .height(TAB_HEIGHT)
        .padding_x(TAB_PADDING_X)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn tab_label(label: String) -> impl IntoNode {
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
    let node = interaction.install(
        container()
            .role(Role::Button)
            .label(AccessibleName::Static(String::from("Close tab")))
            .layout(LayoutStyle::block().fixed_size(CLOSE_AFFORDANCE_WIDTH, CLOSE_AFFORDANCE_WIDTH))
            .class("rounded-sm")
            .child(close_glyph()),
    );
    node.pointer_down(move |event| {
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

#[allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "the argument list mirrors the upstream API contract"
)]
pub(super) fn build_tab(
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
        .class("rounded-md")
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

use super::content::wrap_with_content;

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
