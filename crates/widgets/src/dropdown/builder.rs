use std::rc::Rc;

use sigil_animation_runtime::presets;
use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Overflow};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::{
    NodeStyleExt, Px, StateHost, StateStyle, Style, StyleAccess, StyleAtom, StyleOverlay,
    StyleVariant, TransitionId,
};

use crate::interaction::InteractionState;

use super::super::shared::{
    Activate, BuilderMutator, activate_pointer_reactive, disabled_now, reactive_style_state,
};
use super::super::transition_seam::install_transition_animators;
use super::popup::{OptionCtx, SURFACE_HOVER, TRIGGER_HEIGHT, build_popup};

/// Selection-change callback channel: the newly selected index.
pub(super) type IndexChange = Rc<dyn Fn(usize) + 'static>;

const DROPDOWN_TRANSITION: TransitionId = presets::SMOOTH;
const DROPDOWN_COMPONENT_ID: u64 = 0xD012_D000;
// Animator-only base slice scanned by `install_transition_animators`.
static DROPDOWN_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(DROPDOWN_TRANSITION)];

static TRIGGER_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-inset"),
    StyleAtom::BorderWidth(Px(1)),
    StyleAtom::BorderToneStep(9),
];
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
    TRIGGER_BASE,
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
    label: String,
    disabled: Option<Signal<bool>>,
    on_change: Option<IndexChange>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    width: Option<f32>,
    height: f32,
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
            label: "Dropdown".to_owned(),
            disabled: None,
            on_change: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            width: None,
            height: TRIGGER_HEIGHT,
        }
    }

    /// Set the accessible name announced for this combobox.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set a reactive disabled signal.
    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// Override the trigger width in logical pixels.
    ///
    /// The default is a fixed 200px, which is what makes a bare dropdown
    /// ignore the width of whatever it is placed in. The popup and its
    /// option rows follow this width so the menu stays flush with the
    /// control that opened it.
    #[must_use]
    pub const fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// Override the trigger height in logical pixels.
    #[must_use]
    pub const fn height(mut self, px: f32) -> Self {
        self.height = px;
        self
    }

    /// Run a callback when the selection changes, receiving the newly
    /// selected index (5/5 Wave 1 T5 — widget callbacks carry their value:
    /// checkbox `Fn(bool)`, slider `Fn(f32)`, `text_field` `Fn(&str)`,
    /// dropdown `Fn(usize)`).
    #[must_use]
    pub fn change(mut self, cb: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Rc::new(cb));
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let enabled = !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let open = signal(false);
        let cursor = signal(self.selected.peek());
        let on_change = self
            .on_change
            .as_ref()
            .map_or_else(|| Rc::new(|_| {}), Rc::clone);

        let selected = self.selected;
        let options = Rc::new(self.options);
        let disabled = self.disabled;

        let ctx = OptionCtx {
            selected,
            open,
            cursor,
            disabled,
            width: self.width,
            trigger_height: self.height,
        };
        // The Styled trait surface targets the TRIGGER — that is the
        // user-visible chrome of the dropdown (the popup is a transient
        // surface, the root is an invisible event sink). Passing the
        // accumulator into `build_trigger` lets `.bg()` / `.rounded()`
        // / `.hover()` shape the trigger appearance directly.
        let user_overlay = self.style_accumulator.take_state_overlay();
        let trigger = build_trigger(
            ctx,
            &options,
            interaction,
            self.style_accumulator,
            user_overlay,
        );
        let popup_options = Rc::clone(&options);
        let popup_on_change = Rc::clone(&on_change);
        let mut root = dropdown_root(ctx, Rc::clone(&options), self.label)
            .child(trigger)
            .children_reactive(move || {
                if ctx.open.get() {
                    vec![build_popup(ctx, &popup_options, &popup_on_change)]
                } else {
                    Vec::new()
                }
            });

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
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn dropdown_root(ctx: OptionCtx, options: Rc<Vec<String>>, label: String) -> NodeBuilder {
    let OptionCtx {
        open,
        cursor,
        disabled,
        width,
        trigger_height,
        ..
    } = ctx;
    let root_layout = width.map_or_else(
        || LayoutStyle::block().fill_width().height(trigger_height),
        |px| LayoutStyle::block().width(px).height(trigger_height),
    );
    container()
        .role(Role::ComboBox)
        .label(label)
        .layout(root_layout)
        .key(Key::Escape, move |ev| {
            ev.stop_propagation();
            open.set(false);
        })
        .key(Key::ArrowDown, move |ev| {
            ev.stop_propagation();
            if disabled_now(disabled) {
                return;
            }
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
            if disabled_now(disabled) {
                return;
            }
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
    user_overlay: StyleOverlay,
) -> NodeBuilder {
    let label_options = Rc::clone(options);
    let OptionCtx {
        selected,
        open,
        cursor,
        disabled,
        width: _,
        trigger_height: _,
    } = ctx;
    let state_style = dropdown_trigger_state_style();
    let mut node = container()
        .role(Role::Button)
        .label("Select option")
        .layout(trigger_layout())
        .class("rounded-md")
        .class(TRIGGER_VARIANT.base())
        .class(DROPDOWN_ANIM_BASE)
        .stateful_style(TRIGGER_VARIANT, user_overlay, move || {
            reactive_style_state(interaction, disabled)
        })
        .pointer_up(move |ev| {
            let cb: Activate = Rc::new(move || {
                open.set(!open.peek());
                cursor.set(selected.peek());
            });
            // Reactive guard: a `disabled` flip between down and up
            // must veto activation on release, matching the Enter
            // handler below (review follow-up 2026-07-13).
            activate_pointer_reactive(ev, interaction, disabled, &cb);
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
        .fill()
        .padding_x(10.0)
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
    // The wrapper takes the row's free space via `grow(1.0)`, so the
    // chevron lands flush against the trailing padding edge at ANY
    // trigger width. This previously used a width computed from the
    // fixed trigger size because `grow` did not distribute free space to
    // an item that had children (fixed earlier on this branch), which
    // left the chevron mid-trigger.
    // `overflow_x(Hidden)` clips labels wider than the slot at the
    // trigger's edge instead of painting past the pill (Fable visual
    // 2026-07-13); giving long options readable short labels remains
    // the caller's concern.
    container()
        .layout(
            LayoutStyle::flex_row()
                .grow(1.0)
                .width(0.0)
                .height(24.0)
                .align_items(AlignItems::Center)
                .overflow_x(Overflow::Hidden),
        )
        .child(
            text(TextContent::Reactive(label_sig))
                .layout(LayoutStyle::block().fill_width())
                .class("text-on-surface"),
        )
}

// Down-triangle chevron (U+25BE) as a content-sized text leaf. An
// earlier raw-text `v` glyph never rasterised because it flowed through
// a 14 px *width-clipped* block, so the fallback was an accent-coloured
// swatch — visible but reading as a "featureless square", not a menu
// affordance. A content-sized leaf (no width clip; §4.5 gives text
// leaves their measured width) paints the real triangle and cannot
// overflow — it is itself the leaf. In-flow flex child: the label
// wrapper's `grow(1.0)` eats the free space, so the chevron sits flush
// at the trailing padding edge whatever the trigger's width.
fn trigger_chevron() -> impl IntoNode {
    text("\u{25BE}")
        .layout(LayoutStyle::block())
        .class("text-sm")
        .class("text-on-surface-muted")
}
