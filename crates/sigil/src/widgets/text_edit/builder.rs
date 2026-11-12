use crate::reactive::Reactive;
use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{NodeBuilder, TextContent};
use crate::reactive::{Signal, signal};
use crate::style::Style;

use crate::widgets::interaction::InteractionState;

use super::super::layout_ext::LayoutOverrides;
use super::handlers::{TextEditRuntime, TextEditState, attach_key_handlers, noop_change};
use super::ime::attach_ime_handlers;
use super::render::{build_base_node, content_padding, edit_layout};
use crate::clipboard::{Clipboard, clipboard};

pub(super) use crate::widgets::shared::TextChange;
use crate::widgets::shared::caller_style;

/// The key that runs `TextEditBuilder::submit`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubmitKey {
    /// Ctrl+Enter submits and Enter inserts a newline.
    #[default]
    CtrlEnter,
    /// Enter submits; Shift+Enter, Alt+Enter, and Ctrl+J insert a newline.
    Enter,
}

/// Builder of a text editor.
#[must_use]
pub struct TextEditBuilder {
    pub(super) value: Signal<String>,
    pub(super) placeholder: TextContent,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) change: Option<TextChange>,
    pub(super) submit: Option<TextChange>,
    submit_key: SubmitKey,
    pub(super) clipboard: Option<Rc<dyn Clipboard>>,
    pub(super) caret_visible: Option<Signal<bool>>,
    pub(super) caret_position: Option<Signal<usize>>,
    caret_rect: Option<Signal<Option<Rect>>>,
    pub(super) content_padding: (f32, f32),
    pub(super) chrome: bool,
    state: Option<TextEditState>,
    pub(super) style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

impl core::fmt::Debug for TextEditBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TextEditBuilder")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("submit_key", &self.submit_key)
            .field("caret_visible", &self.caret_visible)
            .field("caret_position", &self.caret_position)
            .field("caret_rect", &self.caret_rect)
            .field("content_padding", &self.content_padding)
            .field("chrome", &self.chrome)
            .field("state", &self.state)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A multi-line text editor.
pub fn text_edit(value: Signal<String>) -> TextEditBuilder {
    TextEditBuilder::new(value)
}

impl TextEditBuilder {
    /// Creates an editor that edits `value`, with Ctrl+Enter as the submit key.
    pub fn new(value: Signal<String>) -> Self {
        Self {
            value,
            placeholder: TextContent::Static(String::new()),
            disabled: None,
            change: None,
            submit: None,
            submit_key: SubmitKey::CtrlEnter,
            clipboard: None,
            caret_visible: None,
            caret_position: None,
            caret_rect: None,
            content_padding: content_padding(),
            chrome: true,
            state: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// A signal that controls whether the caret is drawn; it starts true.
    pub fn caret_visible(&mut self) -> Signal<bool> {
        *self.caret_visible.get_or_insert_with(|| signal(true))
    }

    /// Binds the caret position, as a byte offset in the text, to `position`.
    pub const fn caret_position(mut self, position: Signal<usize>) -> Self {
        self.caret_position = Some(position);
        self
    }

    /// Writes the rectangle of the caret to `rect`.
    pub const fn caret_rect(mut self, rect: Signal<Option<Rect>>) -> Self {
        self.caret_rect = Some(rect);
        self
    }

    /// Sets the padding between the frame and the text in px.
    pub const fn content_padding(mut self, horizontal: f32, vertical: f32) -> Self {
        self.content_padding = (horizontal, vertical);
        self
    }

    /// Removes the background and the state styles when `enabled` is false; the default is true.
    pub const fn chrome(mut self, enabled: bool) -> Self {
        self.chrome = enabled;
        self
    }

    /// Uses the text, caret, selection, and edit history of `state`, so the app can edit the text.
    pub fn state(mut self, state: TextEditState) -> Self {
        self.value = state.value;
        self.caret_position = Some(state.caret);
        self.state = Some(state);
        self
    }

    /// Sets the text shown while the editor is empty.
    pub fn placeholder(mut self, text: impl Into<TextContent>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Blocks edits while `flag` is true.
    pub fn disabled(mut self, flag: impl Reactive<bool>) -> Self {
        self.disabled = Some(flag.into_signal());
        self
    }

    /// Calls `f` with the text after each edit.
    pub fn change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.change = Some(Rc::new(f));
        self
    }

    /// Sets the key that runs `submit`; the default is `SubmitKey::CtrlEnter`.
    pub const fn submit_on(mut self, key: SubmitKey) -> Self {
        self.submit_key = key;
        self
    }

    /// Calls `f` with the text when the user presses the submit key.
    pub fn submit(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.submit = Some(Rc::new(f));
        self
    }

    /// Replaces the default clipboard that cut, copy, and paste use.
    pub fn clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.clipboard = Some(Rc::new(clipboard));
        self
    }

    fn root(mut self) -> NodeBuilder {
        let value = self.value;
        let interaction = InteractionState::new(true).disabled(self.disabled);
        let state = self.state.unwrap_or_else(|| {
            let caret = self.caret_position;
            TextEditState::new(value, caret.unwrap_or_else(|| signal(value.peek().len())))
        });
        let runtime = TextEditRuntime::new(
            state,
            self.disabled,
            self.change.unwrap_or_else(noop_change),
            self.submit.unwrap_or_else(noop_change),
            self.clipboard.unwrap_or_else(clipboard),
            self.caret_visible.unwrap_or_else(|| signal(true)),
            self.content_padding,
        );
        if let Some(target) = self.caret_rect {
            runtime.ime.report_caret_rect(target);
        }
        let overlay = self.style_accumulator.take_state_overlay();
        let node = build_base_node(
            &runtime,
            &self.placeholder,
            interaction,
            overlay,
            self.chrome,
        );
        let node = interaction.install(attach_key_handlers(node, runtime.clone(), self.submit_key));
        let node = attach_ime_handlers(node, &runtime);
        caller_style(node, self.style_accumulator).with(edit_layout())
    }
}

builder_impls!([] TextEditBuilder: paint(style_accumulator), widget, states(style_accumulator), node);

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
