use std::rc::Rc;

use sigil_animation_runtime::presets;
use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{
    ClassSource, NodeStyleExt, SizeToken, StateHost, StateStyle, Style, StyleAccess, StyleAtom,
    StyleVariant, Theme, TransitionId,
};

use crate::interaction::InteractionState;

use super::super::shared::{Activate, BuilderMutator, activate_key, activate_pointer};
use super::super::transition_seam::install_transition_animators;

const TABS_TRANSITION: TransitionId = presets::SMOOTH;
/// Component id for tabs container — per-tab ids OR-in the tab index.
const TABS_COMPONENT_ID: u64 = 0x7AB5_0000;
const TAB_HEIGHT: f32 = SizeToken::TabHeight.px();
const TAB_WIDTH: f32 = SizeToken::TabWidth.px();
const TAB_PADDING: f32 = SizeToken::PaddingSm.px();

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

/// Builder returned by [`tabs`].
pub struct TabsBuilder {
    active: Signal<usize>,
    labels: Vec<String>,
    disabled: Vec<usize>,
    aria_label: String,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
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
        Self {
            active,
            labels,
            disabled: Vec::new(),
            aria_label: String::new(),
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
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
    pub fn style_atoms(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    /// Attach a scoped theme to the tabs root.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let total = self.labels.len();
        let active = self.active;
        let disabled = Rc::new(self.disabled);
        let labels = self.labels;

        let mut root = container()
            .role(Role::TabList)
            .label(accessible_label(&self.aria_label))
            .layout(tab_list_layout(total));

        for (index, label) in labels.into_iter().enumerate() {
            let tab = build_tab(index, label, active, &disabled);
            root = root.child(tab);
        }

        for mutate in self.mutators {
            root = mutate(root);
        }

        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root
    }
}

impl StyleAccess for TabsBuilder {
    fn style(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for TabsBuilder {}

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

fn tab_list_layout(count: usize) -> LayoutStyle {
    // Width = N tabs × TAB_WIDTH. Caller can override via `.class()` or
    // the styled accumulator when a flexible layout is needed. Cast via
    // u16 first — tab counts always fit in 16 bits, and u16 → f32 is
    // exact (mantissa is 23 bits wide).
    let width = f32::from(u16::try_from(count).unwrap_or(u16::MAX)) * TAB_WIDTH;
    LayoutStyle::flex_row()
        .fixed_size(width, TAB_HEIGHT)
        .align_items(AlignItems::Center)
}

fn build_tab(
    index: usize,
    label: String,
    active: Signal<usize>,
    disabled: &Rc<Vec<usize>>,
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

    let activate: Activate = Rc::new(move || {
        if !is_disabled {
            active.set(index);
        }
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
        .style_reactive(move || variant.state(interaction.style_state()))
        .pointer_up(move |event| activate_pointer(event, interaction, &pointer_activate))
        .key(Key::Enter, move |event| {
            activate_key(event, interaction, &key_activate);
        })
        .child(tab_label(label));

    node = install_transition_animators(node, base_atoms, state_style, component_id, interaction);
    interaction.install(node)
}

fn tab_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(TAB_WIDTH, TAB_HEIGHT)
        .padding(TAB_PADDING)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn tab_label(label: String) -> impl IntoNode {
    let width = 2.0_f32.mul_add(-TAB_PADDING, TAB_WIDTH).max(1.0);
    text(label)
        .layout(LayoutStyle::block().width(width))
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
