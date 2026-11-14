use std::rc::Rc;

use crate::clipboard::{self, Clipboard};
use crate::geometry::Rect;
use crate::layout::Length;
use crate::node::{Cx, IntoNode, NodeBuilder, NodeId, TextContent};
use crate::reactive::{Reactive, Signal, memo, signal};
use crate::text::TextStyle;
use crate::widgets::shared::{TextInputProps, noop_change};
use crate::widgets::{Framed, SubmitKey, TextState, Widget, WidgetFrame};

use super::super::text_edit::{MultilineConfig, build_multiline};
use super::adorn::{Adornments, FocusSlot, clear_button};
use super::handlers::{TextFieldRuntime, attach_key_handlers};
use super::render::{FieldInputs, build_base_node, field_layout};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldMode {
    SingleLine,
    Multiline,
}

/// Builder of a text input.
#[must_use]
pub struct TextFieldBuilder {
    state: TextState,
    input: TextInputProps,
    mode: FieldMode,
    multiline: MultilineConfig,
    caret_rect: Option<Signal<Option<Rect>>>,
    invalid: Option<Signal<bool>>,
    scroll_offset: Option<Signal<f32>>,
    password: bool,
    reveal: Option<Signal<bool>>,
    frame: WidgetFrame,
    adorn: Adornments,
    clearable: bool,
    focus_target: FocusSlot,
}

impl core::fmt::Debug for TextFieldBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TextFieldBuilder")
            .field("state", &self.state)
            .field("input", &self.input)
            .field("mode", &self.mode)
            .field("multiline", &self.multiline)
            .field("caret_rect", &self.caret_rect)
            .field("invalid", &self.invalid)
            .field("scroll_offset", &self.scroll_offset)
            .field("password", &self.password)
            .field("reveal", &self.reveal)
            .field("frame", &self.frame)
            .field("clearable", &self.clearable)
            .finish_non_exhaustive()
    }
}

/// A single-line or multi-line text input.
pub fn text_field(input: impl Into<TextState>) -> TextFieldBuilder {
    TextFieldBuilder::new(input)
}

impl TextFieldBuilder {
    /// Creates an input over `input`.
    pub(crate) fn new(input: impl Into<TextState>) -> Self {
        let state = input.into();
        let props = TextInputProps::new(state.value());
        Self {
            state,
            input: props,
            mode: FieldMode::SingleLine,
            multiline: MultilineConfig::default(),
            caret_rect: None,
            invalid: None,
            scroll_offset: None,
            password: false,
            reveal: None,
            frame: WidgetFrame::new(),
            adorn: Adornments::default(),
            clearable: false,
            focus_target: FocusSlot::default(),
        }
    }

    /// Switches the input to a multi-line text editor.
    pub const fn multiline(mut self) -> Self {
        self.mode = FieldMode::Multiline;
        self
    }

    /// Sets the key that runs `submit` in multiline mode; the default is `SubmitKey::CtrlEnter`.
    pub const fn submit_on(mut self, key: SubmitKey) -> Self {
        self.multiline.submit_key = key;
        self
    }

    /// Binds whether the caret is drawn to `visible`.
    pub const fn caret_visible(mut self, visible: Signal<bool>) -> Self {
        self.input.caret_visible(visible);
        self
    }

    /// Binds the caret position, as a byte offset in the text, to `position`.
    pub fn caret(mut self, position: Signal<usize>) -> Self {
        self.state.rebind_caret(position);
        self
    }

    /// Writes the rectangle of the caret to `rect`.
    pub const fn caret_rect(mut self, rect: Signal<Option<Rect>>) -> Self {
        self.caret_rect = Some(rect);
        self
    }

    /// Sets the padding between the frame and the text in px for multiline mode.
    pub const fn content_padding(mut self, horizontal: f32, vertical: f32) -> Self {
        self.multiline.content_padding = Some((horizontal, vertical));
        self
    }

    /// Removes the background and the state styles when `enabled` is false in multiline mode; the default is true.
    pub const fn chrome(mut self, enabled: bool) -> Self {
        self.multiline.chrome = enabled;
        self
    }

    /// Binds the horizontal scroll offset of the text to `offset`.
    pub const fn scroll_offset(mut self, offset: Signal<f32>) -> Self {
        self.scroll_offset = Some(offset);
        self
    }

    /// Sets the text shown while the field is empty.
    pub fn placeholder(mut self, text: impl Into<TextContent>) -> Self {
        self.input.placeholder(text);
        self
    }

    /// Blocks edits while `flag` is true.
    pub fn disabled(mut self, flag: impl Reactive<bool>) -> Self {
        self.input.disabled(flag);
        self
    }

    /// Marks the field invalid while `invalid` is true, in paint and for screen readers; each call adds a condition.
    pub fn invalid(mut self, invalid: impl Reactive<bool>) -> Self {
        let added = invalid.into_signal();
        self.invalid = Some(match self.invalid {
            Some(before) => memo(move || before.get() || added.get()),
            None => added,
        });
        self
    }

    /// Calls `f` with the text after each edit; each call adds a callback.
    pub fn change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.input.add_change(f);
        self
    }

    /// Calls `f` with the text when Enter is pressed; each call adds a callback.
    pub fn submit(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.input.add_submit(f);
        self
    }

    /// Replaces the default clipboard that cut, copy, and paste use.
    pub fn clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.input.clipboard(clipboard);
        self
    }

    /// Shows each character as `*` unless the `reveal` signal is true.
    pub const fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Shows the text of a password field while `show` is true.
    pub const fn reveal(mut self, show: Signal<bool>) -> Self {
        self.reveal = Some(show);
        self
    }

    /// Shows `node` inside the field before the text.
    pub fn leading(mut self, node: impl IntoNode + 'static) -> Self {
        self.adorn.leading = Some(Box::new(node));
        self
    }

    /// Shows `node` inside the field after the text.
    pub fn trailing(mut self, node: impl IntoNode + 'static) -> Self {
        self.adorn.trailing = Some(Box::new(node));
        self
    }

    /// Shows a button that clears the text and focuses the field, while the text is not empty.
    pub const fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    fn runtime(&mut self, column: f32) -> TextFieldRuntime {
        TextFieldRuntime {
            state: self.state.clone(),
            disabled: self.input.disabled,
            change: self.input.change.take().unwrap_or_else(noop_change),
            submit: self.input.submit.take().unwrap_or_else(noop_change),
            clipboard: self
                .input
                .clipboard
                .take()
                .unwrap_or_else(clipboard::current),
            caret_visible: self.input.caret_visible.unwrap_or_else(|| signal(true)),
            text_style: signal(TextStyle::default()),
            scroll_x: self.scroll_offset.unwrap_or_else(|| signal(0.0_f32)),
            column,
            clip_fail: signal(false),
            focused: signal(false),
            preedit: signal(String::new()),
            preedit_cursor: signal(None),
            caret_rect: self.caret_rect,
            password: self.password,
            reveal: self.reveal.unwrap_or_else(|| signal(false)),
        }
    }

    pub(super) fn into_node_builder(mut self) -> NodeBuilder {
        if self.mode == FieldMode::Multiline {
            return build_multiline(
                self.input,
                self.state,
                self.caret_rect,
                self.multiline,
                self.frame,
            );
        }
        let width = match self.frame.width() {
            Some(Length::Px(px)) => Some(px),
            _ => None,
        };
        let runtime = self.runtime(super::selection::column_width(width));
        let clip_fail = runtime.clip_fail;
        let invalid_flag = self.invalid;
        let invalid = memo(move || invalid_flag.is_some_and(|flag| flag.get()) || clip_fail.get());
        let mut adorn = std::mem::take(&mut self.adorn);
        let target = Rc::clone(&self.focus_target);
        if self.clearable {
            adorn.clear = Some(clear_button(&runtime, Rc::clone(&target)));
        }
        let node = build_base_node(
            FieldInputs { invalid, width },
            &self.input.placeholder,
            runtime.clone(),
            adorn,
        );
        let node = attach_key_handlers(node, &runtime).added(move |_, id| target.set(Some(id)));
        self.frame.finish(node.layout(field_layout(width)))
    }
}

impl Framed for TextFieldBuilder {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for TextFieldBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}
