use std::rc::Rc;

use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::{NodeStyleExt, StateStyle, StyleAtom, StyleVariant};

use crate::interaction::InteractionState;

use super::super::shared::{
    Activate, BuilderMutator, activate_pointer, disabled_now, noop_activate,
};
use super::popup::{OptionCtx, SURFACE, SURFACE_HOVER, TRIGGER_HEIGHT, TRIGGER_WIDTH, build_popup};

static TRIGGER_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(8_800),
];
static TRIGGER_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];

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
    pub fn on_change(mut self, cb: impl Fn() + 'static) -> Self {
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
        let trigger = build_trigger(ctx, &options, interaction);
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

impl IntoNode for DropdownBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
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
        .aria_label("Dropdown")
        .layout(LayoutStyle::block().fixed_size(TRIGGER_WIDTH, TRIGGER_HEIGHT))
        .on_key(Key::Escape, move |ev| {
            ev.stop_propagation();
            open.set(false);
        })
        .on_key(Key::ArrowDown, move |ev| {
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
        .on_key(Key::ArrowUp, move |ev| {
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
) -> NodeBuilder {
    let label_options = Rc::clone(options);
    let OptionCtx {
        selected,
        open,
        cursor,
        disabled,
    } = ctx;
    container()
        .role(Role::Button)
        .aria_label("Select option")
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(TRIGGER_WIDTH, TRIGGER_HEIGHT)
                .padding_px(8.0),
        )
        .class("rounded-md")
        .class(TRIGGER_VARIANT.base())
        .style_reactive(move || {
            let mut state = interaction.style_state();
            state.disabled |= disabled_now(disabled);
            TRIGGER_VARIANT.state(state)
        })
        .on_pointer_up(move |ev| {
            let cb: Activate = Rc::new(move || {
                open.set(!open.peek());
                cursor.set(selected.peek());
            });
            activate_pointer(ev, interaction, &cb);
        })
        .on_key(Key::Enter, move |ev| {
            if interaction.activate_key(ev) && !disabled_now(disabled) {
                open.set(!open.peek());
                cursor.set(selected.peek());
            }
        })
        .child(trigger_label(selected, label_options))
}

fn trigger_label(selected: Signal<usize>, options: Rc<Vec<String>>) -> impl IntoNode {
    let label_sig = derived_eq(move || options.get(selected.get()).cloned().unwrap_or_default());
    text(TextContent::Reactive(label_sig))
        .layout(LayoutStyle::block().width_px(TRIGGER_WIDTH - 24.0))
        .class("text-on-surface")
}
