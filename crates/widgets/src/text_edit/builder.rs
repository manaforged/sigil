use std::rc::Rc;

use sigil_kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use sigil_layout::prelude::LayoutBuilderExt;
use sigil_reactive::{Signal, signal};
use sigil_style::{ClassSource, NodeStyleExt, StateHost, Style, StyleAccess};

use crate::interaction::InteractionState;

use super::super::clipboard::{Clipboard, clipboard};
use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{BuilderMutator, disabled_now};
use super::handlers::{TextEditRuntime, attach_key_handlers, noop_change};
use super::ime::attach_ime_handlers;
use super::render::{build_base_node, edit_layout};

pub(super) type TextChange = Rc<dyn Fn(&str) + 'static>;

/// Builder returned by [`text_edit`].
///
/// Mirrors [`TextFieldBuilder`](super::super::text_field::TextFieldBuilder)
/// for the multi-line case: same factory shape, same callback signatures,
/// same layout-override surface. The two widgets share an editing
/// primitive but render differently.
pub struct TextEditBuilder {
    pub(super) value: Signal<String>,
    pub(super) placeholder: TextContent,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) change: Option<TextChange>,
    pub(super) submit: Option<TextChange>,
    pub(super) clipboard: Option<Rc<dyn Clipboard>>,
    pub(super) caret_visible: Option<Signal<bool>>,
    pub(super) style_accumulator: Style,
    mutators: Vec<BuilderMutator>,
    layout_overrides: LayoutOverrides,
}

/// Create a text editor widget over a newline-preserving value signal.
///
/// Pair with [`text_field`](super::super::text_field::text_field) for
/// strictly single-line input.
///
/// # Multi-line rendering
///
/// The bound `Signal<String>` preserves newline characters; the
/// visible line row count rebuilds reactively against the rope via
/// `NodeBuilder::children_reactive` (B1). Typing `Enter` after build
/// grows the on-screen row count without requiring a full tree rebuild.
/// `↑` / `↓` move the caret by line. `PageUp` / `PageDown` move ten lines.
#[must_use]
pub fn text_edit(value: Signal<String>) -> TextEditBuilder {
    TextEditBuilder::new(value)
}

impl TextEditBuilder {
    /// Create a text editor builder.
    #[must_use]
    pub fn new(value: Signal<String>) -> Self {
        Self {
            value,
            placeholder: TextContent::Static(String::new()),
            disabled: None,
            change: None,
            submit: None,
            clipboard: None,
            caret_visible: None,
            style_accumulator: Style::new(),
            mutators: Vec::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Add utility classes or atoms after the built-in text-edit defaults.
    ///
    /// Mirrors the inherent `.class()` on
    /// [`crate::TextFieldBuilder`] and `ButtonBuilder`. Classes
    /// install AFTER the editor's own base atoms (DR-049 §10 additive
    /// composition).
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Signal that tracks whether the text cursor should be rendered.
    pub fn caret_visible(&mut self) -> Signal<bool> {
        *self.caret_visible.get_or_insert_with(|| signal(true))
    }

    /// Set placeholder text rendered while the buffer is empty and
    /// unfocused.
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

    /// Run a callback after user editing changes the value.
    #[must_use]
    pub fn change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.change = Some(Rc::new(f));
        self
    }

    /// Run a callback when `Ctrl+Enter` is pressed. Bare `Enter` always
    /// inserts a newline.
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

    fn into_node_builder(self) -> NodeBuilder {
        let value = self.value;
        let disabled = self.disabled;
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
        let runtime =
            TextEditRuntime::new(value, disabled, change, submit, clipboard, caret_visible);

        // Drain user-supplied state-slot overlays before the base
        // accumulator attaches — the edit frame resolves them against
        // its live interaction slot so `.hover(|s| s.bg(...))` paints.
        let user_overlay = style_accumulator.take_state_overlay();
        let frame_h = layout_overrides
            .height_hint()
            .unwrap_or(super::render::DEFAULT_HEIGHT);
        let node = build_base_node(&runtime, &placeholder, interaction, user_overlay, frame_h);
        let node = interaction.install(attach_key_handlers(node, runtime.clone()));
        let mut node = attach_ime_handlers(node, &runtime);
        for mutate in mutators {
            node = mutate(node);
        }
        if !style_accumulator.is_empty() {
            node = node.style(style_accumulator);
        }
        node.layout(layout_overrides.apply(edit_layout()))
    }
}

impl StyleAccess for TextEditBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for TextEditBuilder {}

impl WidgetLayoutExt for TextEditBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for TextEditBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}
