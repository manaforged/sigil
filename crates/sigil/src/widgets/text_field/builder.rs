use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use crate::reactive::{Signal, memo, signal};
use crate::style::Style;

use crate::widgets::interaction::InteractionState;

use super::super::layout_ext::LayoutOverrides;
use super::super::text_edit::{TextEditRuntime, TextEditState, noop_change};
use super::adorn::{Adornments, FocusSlot, clear_button};
use super::handlers::{TextFieldRuntime, attach_key_handlers};
use super::render::{FieldInputs, build_base_node, field_layout};
use crate::clipboard::{Clipboard, clipboard};

use crate::widgets::shared::{TextInputProps, caller_style};

/// Builder of a text input.
#[must_use]
pub struct TextFieldBuilder {
    pub(super) input: TextInputProps,
    pub(super) invalid: Option<Signal<bool>>,
    pub(super) scroll_offset: Option<Signal<f32>>,
    focus_trigger: Option<Signal<u64>>,
    autofocus: bool,
    pub(super) password: bool,
    pub(super) reveal: Option<Signal<bool>>,
    pub(super) style_accumulator: Style,
    layout_overrides: LayoutOverrides,
    adorn: Adornments,
    clearable: bool,
    focus_target: FocusSlot,
}

impl core::fmt::Debug for TextFieldBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TextFieldBuilder")
            .field("input", &self.input)
            .field("invalid", &self.invalid)
            .field("scroll_offset", &self.scroll_offset)
            .field("focus_trigger", &self.focus_trigger)
            .field("autofocus", &self.autofocus)
            .field("password", &self.password)
            .field("reveal", &self.reveal)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .field("clearable", &self.clearable)
            .finish_non_exhaustive()
    }
}

/// A single-line text input.
pub fn text_field(value: Signal<String>) -> TextFieldBuilder {
    TextFieldBuilder::new(value)
}

impl TextFieldBuilder {
    /// Creates a single-line input that edits `value`.
    pub(crate) fn new(value: Signal<String>) -> Self {
        Self {
            input: TextInputProps::new(value),
            invalid: None,
            scroll_offset: None,
            focus_trigger: None,
            autofocus: false,
            password: false,
            reveal: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
            adorn: Adornments::default(),
            clearable: false,
            focus_target: FocusSlot::default(),
        }
    }

    /// Binds whether the caret is drawn to `visible`.
    pub const fn caret_visible(mut self, visible: Signal<bool>) -> Self {
        self.input.caret_visible(visible);
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
        let input = TextEditRuntime::new(
            TextEditState::new(self.input.value, signal(self.input.value.peek().len())),
            self.input.disabled,
            self.input.change.take().unwrap_or_else(noop_change),
            self.input.submit.take().unwrap_or_else(noop_change),
            self.input.clipboard.take().unwrap_or_else(clipboard),
            self.input.caret_visible.unwrap_or_else(|| signal(true)),
            (0.0, 0.0),
        )
        .single_line(
            column,
            self.scroll_offset.unwrap_or_else(|| signal(0.0_f32)),
        );
        TextFieldRuntime {
            input,
            password: self.password,
            reveal: self.reveal.unwrap_or_else(|| signal(false)),
        }
    }

    pub(super) fn into_node_builder(mut self) -> (NodeBuilder, TextFieldRuntime) {
        let width = self.layout_overrides.width_hint();
        let runtime = self.runtime(super::selection::column_width(width));
        let interaction = InteractionState::new(true).disabled(self.input.disabled);
        let clip_fail = runtime.input.clip_fail;
        let invalid = self.invalid;
        let inputs = FieldInputs {
            invalid: Some(memo(move || {
                invalid.is_some_and(|flag| flag.get()) || clip_fail.get()
            })),
            overlay: self.style_accumulator.take_state_overlay(),
            width,
        };
        let mut adorn = std::mem::take(&mut self.adorn);
        if self.clearable {
            adorn.clear = Some(clear_button(&runtime, Rc::clone(&self.focus_target)));
        }
        let node = build_base_node(
            inputs,
            &self.input.placeholder,
            interaction,
            runtime.clone(),
            adorn,
        );
        let node = interaction.install(attach_key_handlers(node, &runtime));
        let node = caller_style(node, self.style_accumulator);
        let overrides = std::mem::take(&mut self.layout_overrides);
        (overrides.finish(node.with(field_layout(width))), runtime)
    }
}

builder_impls!([] TextFieldBuilder: paint(style_accumulator), widget(focus = autofocus / focus_trigger), states(style_accumulator));

impl IntoNode for TextFieldBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        let focus_trigger = self.focus_trigger;
        let autofocus = self.autofocus;
        let target = Rc::clone(&self.focus_target);
        let (node, _) = self.into_node_builder();
        let node = if autofocus {
            node.with(crate::events::focus::Autofocus)
        } else {
            node
        };
        let id = node.build(cx);
        target.set(Some(id));
        if autofocus {
            id.focus();
        }
        if let Some(trigger) = focus_trigger {
            crate::events::focus::focus_on_trigger(trigger, move || {
                id.focus();
            });
        }
        id
    }
}
