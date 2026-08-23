use std::rc::Rc;

use sigil_kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use sigil_layout::prelude::LayoutBuilderExt;
use sigil_reactive::{Signal, derived, signal};
use sigil_style::{ClassSource, NodeStyleExt, StateHost, Style, StyleAccess};

use crate::interaction::InteractionState;

use super::super::clipboard::{Clipboard, clipboard};
use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{BuilderMutator, disabled_now};
use super::handlers::{TextFieldRuntime, attach_key_handlers, noop_change};
use super::ime::attach_ime_handlers;
use super::render::{FieldInputs, build_base_node, field_layout};

pub(super) type TextChange = Rc<dyn Fn(&str) + 'static>;

/// Builder returned by [`text_field`].
pub struct TextFieldBuilder {
    pub(super) value: Signal<String>,
    pub(super) placeholder: TextContent,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) invalid: Option<Signal<bool>>,
    pub(super) change: Option<TextChange>,
    pub(super) submit: Option<TextChange>,
    pub(super) clipboard: Option<Rc<dyn Clipboard>>,
    pub(super) caret_visible: Option<Signal<bool>>,
    pub(super) scroll_offset: Option<Signal<f32>>,
    focus_trigger: Option<Signal<u64>>,
    pub(super) style_accumulator: Style,
    mutators: Vec<BuilderMutator>,
    layout_overrides: LayoutOverrides,
}

/// Create a single-line text field widget.
#[must_use]
pub fn text_field(value: Signal<String>) -> TextFieldBuilder {
    TextFieldBuilder::new(value)
}

impl TextFieldBuilder {
    /// Create a text field builder.
    #[must_use]
    pub fn new(value: Signal<String>) -> Self {
        Self {
            value,
            placeholder: TextContent::Static(String::new()),
            disabled: None,
            invalid: None,
            change: None,
            submit: None,
            clipboard: None,
            caret_visible: None,
            scroll_offset: None,
            focus_trigger: None,
            style_accumulator: Style::new(),
            mutators: Vec::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Add utility classes or atoms after the built-in text-field defaults.
    ///
    /// Mirrors the inherent `.class()` on [`crate::ButtonBuilder`].
    /// Classes are installed AFTER the field's own base atoms so user
    /// classes override the defaults (per DR-049 additive composition).
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Signal that tracks whether the text cursor should be rendered.
    ///
    /// Starts `true`; reset to `true` on every keystroke. An external timer
    /// (e.g., a 500 ms interval on the shell event loop) toggles it to create
    /// the blinking effect. Tests can read this to assert the reset behavior.
    pub fn caret_visible(&mut self) -> Signal<bool> {
        *self.caret_visible.get_or_insert_with(|| signal(true))
    }

    /// Horizontal scroll of the text in logical px (left edge of the visible
    /// viewport).
    ///
    /// Updated after every caret movement so the caret stays in view; pixel-,
    /// not character-, based so it is correct at any field width and font.
    /// Tests can read this to assert scroll behavior after long input.
    pub fn scroll_offset(&mut self) -> Signal<f32> {
        *self.scroll_offset.get_or_insert_with(|| signal(0.0_f32))
    }

    /// Set placeholder text rendered while the field is empty.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<TextContent>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Set a reactive disabled signal.
    #[must_use]
    pub const fn disabled(mut self, flag: Signal<bool>) -> Self {
        self.disabled = Some(flag);
        self
    }

    /// Bind a reactive invalid signal that paints a red border when `true`.
    ///
    /// While the signal peeks `true` the field's outer border is
    /// repainted with a saturated red overlay (Material "error"
    /// #D32F2F) that overrides every other state slot — rest, hover,
    /// focus, pressed, and disabled. Flipping the signal updates the
    /// rendered border on the next reactive tick without rebuilding
    /// the tree.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sigil::widgets::text_field::text_field;
    /// use sigil_reactive::{derived, signal};
    ///
    /// let value = signal(String::new());
    /// let invalid = derived(move || value.get().trim().is_empty());
    /// let node = text_field(value).invalid_when(invalid);
    /// // Border turns red whenever the field is blank.
    /// ```
    #[must_use]
    pub const fn invalid_when(mut self, invalid: Signal<bool>) -> Self {
        self.invalid = Some(invalid);
        self
    }

    /// Run a callback after user editing changes the value.
    ///
    /// The bound `Signal<String>` is already two-way. Use this for
    /// side effects, not to copy `v` back into the signal.
    #[must_use]
    pub fn change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.change = Some(Rc::new(f));
        self
    }

    /// Run a callback when Enter is pressed.
    #[must_use]
    pub fn submit(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.submit = Some(Rc::new(f));
        self
    }

    /// Provide a platform clipboard implementation.
    #[must_use]
    pub fn clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.clipboard = Some(Rc::new(clipboard));
        self
    }

    /// Focus this field whenever `trigger` changes.
    ///
    /// Signal-first focus API: key/shortcut handlers receive only
    /// `&mut KeyEvent` (no `Cx`), so the app bumps a generation counter
    /// (`trigger.set(trigger.peek() + 1)`) from the handler and the
    /// field requests focus itself. The mount-time read is treated as
    /// steady — the initial value never steals focus. Requests go
    /// through [`sigil_events::request_focus`] and are validated by the
    /// router against the Tab-navigation contract (focusable, laid out,
    /// inside the active focus scope), landing before the next routed
    /// event.
    #[must_use]
    pub const fn focus_when(mut self, trigger: Signal<u64>) -> Self {
        self.focus_trigger = Some(trigger);
        self
    }

    pub(super) fn into_node_builder(self) -> (NodeBuilder, TextFieldRuntime) {
        let value = self.value;
        let disabled = self.disabled;
        let invalid = self.invalid;
        let placeholder = self.placeholder;
        let mut style_accumulator = self.style_accumulator;
        let mutators = self.mutators;
        let layout_overrides = self.layout_overrides;
        let change = self.change.unwrap_or_else(noop_change);
        let submit = self.submit.unwrap_or_else(noop_change);
        let clipboard = self.clipboard.unwrap_or_else(clipboard);

        let enabled = !disabled_now(disabled);
        let interaction = InteractionState::new(enabled);
        let caret_visible = self.caret_visible.unwrap_or_else(|| signal(true));
        let scroll = self.scroll_offset.unwrap_or_else(|| signal(0.0_f32));

        // Read the caller's `.width(px)` (if any) before `apply` consumes the
        // overrides, so the text track and the scroll math both size against
        // the field's real inner column instead of the default.
        let width = layout_overrides.width_hint();
        let height = layout_overrides.height_hint();
        let column = super::selection::column_width(width);
        let runtime = TextFieldRuntime::new(
            value,
            disabled,
            change,
            submit,
            clipboard,
            caret_visible,
            scroll,
            column,
        );
        let clip_fail = runtime.clip_fail;
        let invalid = Some(derived(move || {
            invalid.is_some_and(|flag| flag.get()) || clip_fail.get()
        }));

        // Drain user-supplied state-slot overlays before the base
        // accumulator attaches — the field frame resolves them against
        // its live interaction slot so `.hover(|s| s.bg(...))` paints.
        let user_overlay = style_accumulator.take_state_overlay();
        let inputs = FieldInputs {
            value,
            disabled,
            invalid,
            overlay: user_overlay,
            width,
            height,
        };
        let node = build_base_node(inputs, &placeholder, interaction, runtime.clone());
        let node = interaction.install(attach_key_handlers(node, runtime.clone()));
        let mut node = attach_ime_handlers(node, &runtime);
        for mutate in mutators {
            node = mutate(node);
        }
        // Styled accumulator installs AFTER the field's own atoms +
        // state-variant install so the trait surface layers ON TOP of
        // the FIELD_BASE/FIELD_REST/FIELD_FOCUS/FIELD_HOVER/FIELD_DOWN/
        // FIELD_DISABLED state arms (additive composition per DR-049
        // §10). The accumulator's hover/focus/pressed/disabled slot
        // overlays were drained above into the frame's state overlay;
        // base atoms route through the existing `NodeStyleExt::style`
        // class-source seam.
        if !style_accumulator.is_empty() {
            node = node.style(style_accumulator);
        }
        node = node.layout(layout_overrides.apply(field_layout()));
        (node, runtime)
    }
}

impl StyleAccess for TextFieldBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for TextFieldBuilder {}

impl WidgetLayoutExt for TextFieldBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for TextFieldBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        let focus_trigger = self.focus_trigger;
        let (node, _runtime) = self.into_node_builder();
        let id = node.build(cx);
        if let Some(trigger) = focus_trigger {
            install_focus_trigger(id, trigger);
        }
        id
    }
}

/// Focus-request effect behind [`TextFieldBuilder::focus_when`].
///
/// Mirrors the modal open-announcement idiom (`modal/builder.rs`): the
/// effect observes the trigger, treats the initial read as steady, and
/// pushes through the deferred [`sigil_events::request_focus`] queue on
/// each later change — effects cannot hold `&mut Cx` or reach the
/// router directly. Created inside `build`, so it lives and dies with
/// the node's owner scope.
fn install_focus_trigger(id: sigil_kernel::NodeId, trigger: Signal<u64>) {
    let last = std::cell::Cell::new(trigger.peek());
    sigil_reactive::effect_named("sigil::text_field::focus_when", move || {
        let generation = trigger.get();
        if generation != last.replace(generation) {
            sigil_events::request_focus(id);
        }
    });
}
