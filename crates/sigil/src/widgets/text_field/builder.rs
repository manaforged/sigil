use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use crate::reactive::{Signal, derived, signal};
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

/// Builder of a text input.
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

/// A single-line text input.
#[must_use]
pub fn text_field(value: Signal<String>) -> TextFieldBuilder {
    TextFieldBuilder::new(value)
}

impl TextFieldBuilder {
    /// Creates a text field builder.
    #[must_use]
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
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<TextContent>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, flag: impl Reactive<bool>) -> Self {
        self.disabled = Some(flag.into_signal());
        self
    }

    /// Sets the invalid when.
    #[must_use]
    pub fn invalid_when(mut self, invalid: impl crate::reactive::Reactive<bool>) -> Self {
        self.invalid = Some(invalid.into_signal());
        self
    }

    /// Sets the change.
    #[must_use]
    pub fn change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.change = Some(Rc::new(f));
        self
    }

    /// Sets the submit.
    #[must_use]
    pub fn submit(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.submit = Some(Rc::new(f));
        self
    }

    /// Sets the blur.
    #[must_use]
    pub fn blur(mut self, f: impl Fn() + 'static) -> Self {
        self.blur = Some(Rc::new(f));
        self
    }

    /// Sets the clipboard.
    #[must_use]
    pub fn clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.clipboard = Some(Rc::new(clipboard));
        self
    }

    /// Applies the password option.
    #[must_use]
    pub const fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Sets the reveal.
    #[must_use]
    pub const fn reveal(mut self, show: Signal<bool>) -> Self {
        self.reveal = Some(show);
        self
    }

    /// Sets the leading.
    #[must_use]
    pub fn leading(mut self, node: impl IntoNode + 'static) -> Self {
        self.adorn.leading = Some(Box::new(node));
        self
    }

    /// Sets the trailing.
    #[must_use]
    pub fn trailing(mut self, node: impl IntoNode + 'static) -> Self {
        self.adorn.trailing = Some(Box::new(node));
        self
    }

    /// Applies the clearable option.
    #[must_use]
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
            invalid: Some(derived(move || {
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
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
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
