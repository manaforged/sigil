use std::rc::Rc;

use sigil_kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use sigil_reactive::{Signal, signal};

use crate::interaction::InteractionState;

use super::super::shared::disabled_now;
use super::handlers::{NoopClipboard, TextFieldRuntime, attach_key_handlers, noop_text_change};
use super::render::build_base_node;

pub(super) type TextChange = Rc<dyn Fn(&str) + 'static>;

/// Platform clipboard abstraction. A no-op stub is used in tests.
pub trait Clipboard {
    /// Write `text` to the clipboard.
    fn set(&self, text: &str);
    /// Read the current clipboard contents.
    fn get(&self) -> Option<String>;
}

/// Builder returned by [`text_field`].
pub struct TextFieldBuilder {
    pub(super) value: Signal<String>,
    pub(super) placeholder: TextContent,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) on_change: Option<TextChange>,
    pub(super) on_submit: Option<TextChange>,
    pub(super) clipboard: Option<Rc<dyn Clipboard>>,
    pub(super) caret_visible: Signal<bool>,
    pub(super) scroll_offset: Signal<usize>,
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
            on_change: None,
            on_submit: None,
            clipboard: None,
            caret_visible: signal(true),
            scroll_offset: signal(0_usize),
        }
    }

    /// Signal that tracks whether the text cursor should be rendered.
    ///
    /// Starts `true`; reset to `true` on every keystroke. An external timer
    /// (e.g., a 500 ms interval on the shell event loop) toggles it to create
    /// the blinking effect. Tests can read this to assert the reset behavior.
    #[must_use]
    pub const fn caret_visible(&self) -> Signal<bool> {
        self.caret_visible
    }

    /// Byte offset of the left edge of the visible viewport within the buffer.
    ///
    /// Updated after every caret movement so that the caret stays in view.
    /// Tests can read this to assert scroll behavior after long input.
    #[must_use]
    pub const fn scroll_offset(&self) -> Signal<usize> {
        self.scroll_offset
    }

    /// Set placeholder text rendered while the field is empty and unfocused.
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
    pub fn on_change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    /// Run a callback when Enter is pressed.
    #[must_use]
    pub fn on_submit(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.on_submit = Some(Rc::new(f));
        self
    }

    /// Provide a platform clipboard implementation.
    #[must_use]
    pub fn clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.clipboard = Some(Rc::new(clipboard));
        self
    }

    pub(super) fn into_node_builder(self) -> (NodeBuilder, TextFieldRuntime) {
        let value = self.value;
        let disabled = self.disabled;
        let placeholder = self.placeholder;
        let on_change = self.on_change.unwrap_or_else(noop_text_change);
        let on_submit = self.on_submit.unwrap_or_else(noop_text_change);
        let clipboard = self.clipboard.unwrap_or_else(|| Rc::new(NoopClipboard));

        let enabled = !disabled_now(disabled);
        let interaction = InteractionState::new(enabled);
        let mut runtime = TextFieldRuntime::new(value, disabled, on_change, on_submit, clipboard);
        runtime.caret_visible = self.caret_visible;
        runtime.scroll_offset = self.scroll_offset;

        let node = build_base_node(value, disabled, &placeholder, interaction);
        let node = interaction.install(attach_key_handlers(node, runtime.clone()));
        (node, runtime)
    }
}

impl IntoNode for TextFieldBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        let (node, _runtime) = self.into_node_builder();
        node.build(cx)
    }
}
