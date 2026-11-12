use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use crate::reactive::{Signal, memo, signal};
use crate::style::Style;
use crate::widgets::Widget;

use crate::widgets::interaction::InteractionState;

use super::super::layout_ext::LayoutOverrides;
use super::super::text_edit::{TextEditRuntime, TextEditState, noop_change};
use super::adorn::{Adornments, FocusSlot, clear_button};
use super::handlers::{TextFieldRuntime, attach_key_handlers};
use super::render::{FieldInputs, build_base_node, field_layout};
use crate::clipboard::{Clipboard, clipboard};

pub(super) use crate::widgets::shared::TextChange;
use crate::widgets::shared::caller_style;

/// Builder of a text input.
#[must_use]
pub struct TextFieldBuilder {
    pub(super) value: Signal<String>,
    pub(super) placeholder: TextContent,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) invalid: Option<Signal<bool>>,
    pub(super) change: Option<TextChange>,
    pub(super) submit: Option<TextChange>,
    blur: Option<Rc<dyn Fn() + 'static>>,
    pub(super) clipboard: Option<Rc<dyn Clipboard>>,
    pub(super) caret_visible: Option<Signal<bool>>,
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
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("invalid", &self.invalid)
            .field("caret_visible", &self.caret_visible)
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
    /// Creates a text field builder.
    pub fn new(value: Signal<String>) -> Self {
        Self {
            value,
            placeholder: TextContent::Static(String::new()),
            disabled: None,
            invalid: None,
            change: None,
            submit: None,
            blur: None,
            clipboard: None,
            caret_visible: None,
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

    /// Returns the signal that says whether the caret shows.
    pub fn caret_visible(&mut self) -> Signal<bool> {
        *self.caret_visible.get_or_insert_with(|| signal(true))
    }

    /// Returns the signal that holds the horizontal scroll offset.
    pub fn scroll_offset(&mut self) -> Signal<f32> {
        *self.scroll_offset.get_or_insert_with(|| signal(0.0_f32))
    }

    /// Sets the placeholder.
    pub fn placeholder(mut self, text: impl Into<TextContent>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Sets the disabled.
    pub fn disabled(mut self, flag: impl Reactive<bool>) -> Self {
        self.disabled = Some(flag.into_signal());
        self
    }

    /// Marks the field invalid while `invalid` is true, in paint and for screen readers; each call adds a condition.
    pub fn invalid_when(mut self, invalid: impl Reactive<bool>) -> Self {
        let added = invalid.into_signal();
        self.invalid = Some(match self.invalid {
            Some(before) => memo(move || before.get() || added.get()),
            None => added,
        });
        self
    }

    /// Calls `f` with the text after each edit; each call adds a callback.
    pub fn change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.change = Some(chain(self.change.take(), f));
        self
    }

    /// Calls `f` with the text when Enter is pressed; each call adds a callback.
    pub fn submit(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.submit = Some(chain(self.submit.take(), f));
        self
    }

    /// Calls `f` when the field loses focus; each call adds a callback.
    pub fn blur(mut self, f: impl Fn() + 'static) -> Self {
        self.blur = Some(match self.blur.take() {
            Some(before) => Rc::new(move || {
                before();
                f();
            }),
            None => Rc::new(f),
        });
        self
    }

    /// Sets the clipboard.
    pub fn clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.clipboard = Some(Rc::new(clipboard));
        self
    }

    /// Applies the password option.
    pub const fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Sets the reveal.
    pub const fn reveal(mut self, show: Signal<bool>) -> Self {
        self.reveal = Some(show);
        self
    }

    /// Sets the leading.
    pub fn leading(mut self, node: impl IntoNode + 'static) -> Self {
        self.adorn.leading = Some(Box::new(node));
        self
    }

    /// Sets the trailing.
    pub fn trailing(mut self, node: impl IntoNode + 'static) -> Self {
        self.adorn.trailing = Some(Box::new(node));
        self
    }

    /// Applies the clearable option.
    pub const fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    fn runtime(&mut self, column: f32) -> TextFieldRuntime {
        let input = TextEditRuntime::new(
            TextEditState::new(self.value, signal(self.value.peek().len())),
            self.disabled,
            self.change.take().unwrap_or_else(noop_change),
            self.submit.take().unwrap_or_else(noop_change),
            self.clipboard.take().unwrap_or_else(clipboard),
            self.caret_visible.unwrap_or_else(|| signal(true)),
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
        let interaction = InteractionState::new(true).disabled(self.disabled);
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
            &self.placeholder,
            interaction,
            runtime.clone(),
            adorn,
        );
        let mut node = interaction.install(attach_key_handlers(node, &runtime));
        if let Some(blur) = self.blur {
            node = node.focus_out(move || blur());
        }
        let node = caller_style(node, self.style_accumulator);
        let overrides = std::mem::take(&mut self.layout_overrides);
        (overrides.finish(node.with(field_layout(width))), runtime)
    }
}

fn chain(before: Option<TextChange>, f: impl Fn(&str) + 'static) -> TextChange {
    match before {
        Some(before) => Rc::new(move |text| {
            before(text);
            f(text);
        }),
        None => Rc::new(f),
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
            crate::events::request_focus(id);
        }
        if let Some(trigger) = focus_trigger {
            crate::events::focus::focus_on_trigger(trigger, move || {
                crate::events::request_focus(id);
            });
        }
        id
    }
}
