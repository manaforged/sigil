use std::rc::Rc;

use animation_runtime::presets;
use events::prelude::{Key, NodeEventsExt};
use kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use reactive::{Signal, derived_eq, signal};
use style::{
    NodeStyleExt, StateHost, StateStyle, Style, StyleAccess, StyleAtom, StyleVariant, TransitionId,
};

use crate::interaction::InteractionState;

use super::super::shared::{
    Activate, BuilderMutator, activate_pointer, disabled_now, noop_activate,
};
use super::super::transition_seam::install_transition_animators;
use super::popup::{OptionCtx, SURFACE, SURFACE_HOVER, TRIGGER_HEIGHT, TRIGGER_WIDTH, build_popup};

const DROPDOWN_TRANSITION: TransitionId = presets::SMOOTH;
const DROPDOWN_COMPONENT_ID: u64 = 0xD012_D000;
// Animator-only base slice scanned by `install_transition_animators`.
static DROPDOWN_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(DROPDOWN_TRANSITION)];

static TRIGGER_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed")];
// Disabled paints `surface-pressed-strong` at full opacity. The
// previous design used `surface-hover` + 50 % opacity, but the alpha
// blend back over `bg-surface` produced only an ~11 sRGB delta versus
// the resting state. `surface-pressed-strong` lands ~63 sRGB units
// below the resting `surface` after gamma encoding and ~40 below the
// hover slot's `surface-hover` — well clear of the JND on the
// 200×40 trigger. The token semantically reads as "darker than
// pressed" rather than "faded", but the W5.3 widget-catalog pass
// owns the richer faded look; this fix just buys visible distinction.
static TRIGGER_DISABLED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

static TRIGGER_VARIANT: StyleVariant = StyleVariant::new(
    SURFACE,
    StateStyle::new(
        &[],
        SURFACE_HOVER,
        SURFACE_HOVER,
        TRIGGER_DOWN,
        TRIGGER_DISABLED,
    ),
);

/// Builder returned by [`dropdown`].
pub struct DropdownBuilder {
    selected: Signal<usize>,
    options: Vec<String>,
    disabled: Option<Signal<bool>>,
    on_change: Option<Activate>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

/// Create a dropdown (combobox) widget.
///
/// `selected` holds the index of the currently selected option.
/// `options` is the list of labels shown in the popup.
#[must_use]
pub fn dropdown(selected: Signal<usize>, options: Vec<String>) -> DropdownBuilder {
    DropdownBuilder::new(selected, options)
}

impl DropdownBuilder {
    /// Create a dropdown builder.
    #[must_use]
    pub fn new(selected: Signal<usize>, options: Vec<String>) -> Self {
        Self {
            selected,
            options,
            disabled: None,
            on_change: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
    }

    /// Set a reactive disabled signal.
    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// Run a callback when the selection changes (receives no args — read `selected`).
    #[must_use]
    pub fn change(mut self, cb: impl Fn() + 'static) -> Self {
        self.on_change = Some(Rc::new(cb));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let enabled = !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let open = signal(false);
        let cursor = signal(self.selected.peek());
        let on_change = self
            .on_change
            .as_ref()
            .map_or_else(noop_activate, Rc::clone);

        let selected = self.selected;
        let options = Rc::new(self.options);
        let disabled = self.disabled;

        let ctx = OptionCtx {
            selected,
            open,
            cursor,
            disabled,
        };
        // The Styled trait surface targets the TRIGGER — that is the
        // user-visible chrome of the dropdown (the popup is a transient
        // surface, the root is an invisible event sink). Passing the
        // accumulator into `build_trigger` lets `.bg()` / `.rounded()`
        // / `.hover()` shape the trigger appearance directly.
        let trigger = build_trigger(ctx, &options, interaction, self.style_accumulator);
        let popup = build_popup(ctx, &options, &on_change);
        let mut root = dropdown_root(open, cursor, Rc::clone(&options))
            .child(trigger)
            .child(popup);

        for mutate in self.mutators {
            root = mutate(root);
        }

        interaction.install(root)
    }
}

impl StyleAccess for DropdownBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for DropdownBuilder {}

impl IntoNode for DropdownBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn dropdown_root(
    open: Signal<bool>,
    cursor: Signal<usize>,
    options: Rc<Vec<String>>,
) -> NodeBuilder {
    container()
        .role(Role::ComboBox)
        .label("Dropdown")
        .layout(LayoutStyle::block().fixed_size(TRIGGER_WIDTH, TRIGGER_HEIGHT))
        .key(Key::Escape, move |ev| {
            ev.stop_propagation();
            open.set(false);
        })
        .key(Key::ArrowDown, move |ev| {
            ev.stop_propagation();
            if !open.peek() {
                open.set(true);
                return;
            }
            let n = options.len();
            if n > 0 {
                cursor.set((cursor.peek() + 1).min(n - 1));
            }
        })
        .key(Key::ArrowUp, move |ev| {
            ev.stop_propagation();
            if cursor.peek() > 0 {
                cursor.set(cursor.peek() - 1);
            }
        })
}

fn build_trigger(
    ctx: OptionCtx,
    options: &Rc<Vec<String>>,
    interaction: InteractionState,
    styled: Style,
) -> NodeBuilder {
    let label_options = Rc::clone(options);
    let OptionCtx {
        selected,
        open,
        cursor,
        disabled,
    } = ctx;
    let state_style = dropdown_trigger_state_style();
    let mut node = container()
        .role(Role::Button)
        .label("Select option")
        .layout(trigger_layout())
        .class("rounded-md")
        .class(TRIGGER_VARIANT.base())
        .class(DROPDOWN_ANIM_BASE)
        .style_reactive(move || {
            let mut state = interaction.style_state();
            state.disabled |= disabled_now(disabled);
            TRIGGER_VARIANT.state(state)
        })
        .pointer_up(move |ev| {
            let cb: Activate = Rc::new(move || {
                open.set(!open.peek());
                cursor.set(selected.peek());
            });
            activate_pointer(ev, interaction, &cb);
        })
        .key(Key::Enter, move |ev| {
            if interaction.activate_key(ev) && !disabled_now(disabled) {
                open.set(!open.peek());
                cursor.set(selected.peek());
            }
        })
        .child(trigger_label(selected, label_options))
        .child(trigger_chevron());
    node = install_transition_animators(
        node,
        DROPDOWN_ANIM_BASE,
        state_style,
        DROPDOWN_COMPONENT_ID,
        interaction,
    );
    // Styled accumulator merges into the trigger's class chain LAST so
    // user-supplied tokens layer on top of the variant default.
    if !styled.is_empty() {
        node = node.style(styled);
    }
    node
}

// Push the chevron flush against the right padding edge: SpaceBetween
// distributes free space between the label wrapper (fixed-width child)
// and the chevron, landing the chevron at the trigger's trailing edge.
// Without SpaceBetween, both children pack to the left and the chevron
// renders mid-trigger (the v2 bug).
fn trigger_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(TRIGGER_WIDTH, TRIGGER_HEIGHT)
        .padding(8.0)
        .gap(8.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
}

const fn dropdown_trigger_state_style() -> StateStyle {
    StateStyle::new(
        &[],
        SURFACE_HOVER,
        SURFACE_HOVER,
        TRIGGER_DOWN,
        TRIGGER_DISABLED,
    )
}

fn trigger_label(selected: Signal<usize>, options: Rc<Vec<String>>) -> impl IntoNode {
    let label_sig = derived_eq(move || options.get(selected.get()).cloned().unwrap_or_default());
    // Wrap the text node in a flex_row container so it participates as
    // a flex SIBLING of the chevron under the trigger's flex layout.
    // The wrapper uses `grow(1.0)` instead of a fixed width so the
    // trigger row's `SpaceBetween` justification consumes the free
    // space here and pushes the chevron flush against the trailing
    // padding edge. A `LayoutStyle::block()` text child sits inside
    // the wrapper so the text laying-out behaviour matches the v2
    // single-line block formatting.
    // Fixed width 160 = TRIGGER_WIDTH (200) - padding_x (16) - chevron (16)
    // - gap (8) = 160. Hard-coding the width here (rather than using
    // `grow(1.0)`) makes the SpaceBetween distribution deterministic at
    // the engine level — `grow` interacts badly with stub-measured text
    // children in golden tests, leaving the chevron mid-trigger.
    container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(160.0, 24.0)
                .align_items(AlignItems::Center),
        )
        .child(
            text(TextContent::Reactive(label_sig))
                .layout(LayoutStyle::block().width(160.0))
                .class("text-on-surface"),
        )
}

// Down-arrow chevron — a small filled `accent`-coloured swatch pinned
// to the trigger's right edge. The previous design rendered a Latin
// `v` glyph over a `surface-pressed` swatch, but the single-character
// text payload never reached the rasteriser through the 14 px
// width-clipped block-flow, leaving the swatch reading as
// "featureless grey square". Switching the swatch to the `accent`
// token gives it enough visual weight to read as an affordance on
// its own (high contrast against the trigger's `surface` resting
// background, ~50 sRGB delta minimum across every state slot).
// W5.3 owns the richer glyph-or-icon affordance; this fix just
// restores user-visible "this opens a dropdown" cue.
fn trigger_chevron() -> impl IntoNode {
    static CHEVRON_BG: &[style::StyleAtom] = &[style::StyleAtom::BackgroundThemeRef("accent")];
    // Inset math: trigger is `TRIGGER_WIDTH` wide with 8 px right
    // padding; chevron swatch is 16 px wide. Left inset =
    // TRIGGER_WIDTH - 16 (chevron) - 8 (right padding) = 176. Top
    // inset = (TRIGGER_HEIGHT - 16) / 2 = 12, vertically centred.
    let left = TRIGGER_WIDTH - 16.0 - 8.0;
    let top = (TRIGGER_HEIGHT - 16.0) / 2.0;
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(16.0, 16.0)
                .position(Position::Absolute)
                .inset_left(left)
                .inset_top(top),
        )
        .class("rounded-sm")
        .class(CHEVRON_BG)
}
