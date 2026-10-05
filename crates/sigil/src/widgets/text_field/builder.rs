use crate::reactive::Reactive;
use std::rc::Rc;

use crate::events::NodeEventsExt;
use crate::kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use crate::reactive::{Signal, derived, signal};
use crate::style::{PaintTarget, StateHost, Style, StyleSlot};

use crate::widgets::interaction::InteractionState;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{BuilderMutator, disabled_now};
use super::super::text_edit::{FocusCaret, noop_change};
use super::adorn::{Adornments, FocusTarget, clear_button};
use super::handlers::{FieldConfig, TextFieldRuntime, attach_key_handlers};
use super::render::{FieldInputs, build_base_node, field_layout};
use crate::clipboard::{Clipboard, clipboard};

pub(super) type TextChange = Rc<dyn Fn(&str) + 'static>;

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
    focus_caret: FocusCaret,
    pub(super) password: bool,
    pub(super) reveal: Option<Signal<bool>>,
    pub(super) style_accumulator: Style,
    mutators: Vec<BuilderMutator>,
    layout_overrides: LayoutOverrides,
    adorn: Adornments,
    clearable: bool,
    focus_target: FocusTarget,
}

#[must_use]
pub fn text_field(value: Signal<String>) -> TextFieldBuilder {
    TextFieldBuilder::new(value)
}

impl TextFieldBuilder {
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
            focus_caret: FocusCaret::Keep,
            password: false,
            reveal: None,
            style_accumulator: Style::new(),
            mutators: Vec::new(),
            layout_overrides: LayoutOverrides::new(),
            adorn: Adornments::default(),
            clearable: false,
            focus_target: FocusTarget::default(),
        }
    }

    pub fn caret_visible(&mut self) -> Signal<bool> {
        *self.caret_visible.get_or_insert_with(|| signal(true))
    }

    pub fn scroll_offset(&mut self) -> Signal<f32> {
        *self.scroll_offset.get_or_insert_with(|| signal(0.0_f32))
    }

    #[must_use]
    pub fn placeholder(mut self, text: impl Into<TextContent>) -> Self {
        self.placeholder = text.into();
        self
    }

    #[must_use]
    pub fn disabled(mut self, flag: impl Reactive<bool>) -> Self {
        self.disabled = Some(flag.into_signal());
        self
    }

    #[must_use]
    pub fn invalid_when(mut self, invalid: impl crate::reactive::Reactive<bool>) -> Self {
        self.invalid = Some(invalid.into_signal());
        self
    }

    #[must_use]
    pub fn change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.change = Some(Rc::new(f));
        self
    }

    #[must_use]
    pub fn submit(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.submit = Some(Rc::new(f));
        self
    }

    #[must_use]
    pub fn blur(mut self, f: impl Fn() + 'static) -> Self {
        self.blur = Some(Rc::new(f));
        self
    }

    #[must_use]
    pub fn clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.clipboard = Some(Rc::new(clipboard));
        self
    }

    #[must_use]
    pub const fn focus_when(mut self, trigger: Signal<u64>) -> Self {
        self.focus_trigger = Some(trigger);
        self
    }

    #[must_use]
    pub const fn autofocus(mut self) -> Self {
        self.autofocus = true;
        self
    }

    #[must_use]
    pub const fn focus_caret(mut self, at: FocusCaret) -> Self {
        self.focus_caret = at;
        self
    }

    #[must_use]
    pub const fn password(mut self) -> Self {
        self.password = true;
        self
    }

    #[must_use]
    pub const fn reveal(mut self, show: Signal<bool>) -> Self {
        self.reveal = Some(show);
        self
    }

    #[must_use]
    pub fn leading(mut self, node: impl IntoNode + 'static) -> Self {
        self.adorn.leading = Some(Box::new(node));
        self
    }

    #[must_use]
    pub fn trailing(mut self, node: impl IntoNode + 'static) -> Self {
        self.adorn.trailing = Some(Box::new(node));
        self
    }

    #[must_use]
    pub const fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    fn runtime(&mut self, column: f32) -> TextFieldRuntime {
        TextFieldRuntime::new(FieldConfig {
            value: self.value,
            disabled: self.disabled,
            change: self.change.take().unwrap_or_else(noop_change),
            submit: self.submit.take().unwrap_or_else(noop_change),
            clipboard: self.clipboard.take().unwrap_or_else(clipboard),
            caret_visible: self.caret_visible.unwrap_or_else(|| signal(true)),
            scroll: self.scroll_offset.unwrap_or_else(|| signal(0.0_f32)),
            column,
            password: self.password,
            reveal: self.reveal.unwrap_or_else(|| signal(false)),
        })
    }

    pub(super) fn into_node_builder(mut self) -> (NodeBuilder, TextFieldRuntime) {
        let width = self.layout_overrides.width_hint();
        let height = self.layout_overrides.height_hint();
        let runtime = self.runtime(super::selection::column_width(width));
        let interaction = InteractionState::new(!disabled_now(self.disabled));
        let clip_fail = runtime.input.clip_fail;
        let invalid = self.invalid;
        let inputs = FieldInputs {
            value: self.value,
            disabled: self.disabled,
            invalid: Some(derived(move || {
                invalid.is_some_and(|flag| flag.get()) || clip_fail.get()
            })),
            overlay: self.style_accumulator.take_state_overlay(),
            width,
            height,
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
        let node = interaction.install(attach_key_handlers(node, &runtime));
        let node = match self.blur {
            Some(blur) => node.focus_out(move || blur()),
            None => node,
        };
        let mut node = self
            .mutators
            .into_iter()
            .fold(node, |node, mutate| mutate(node));
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        let hooks = self.layout_overrides.hooks();
        let node = node.with(self.layout_overrides.apply(field_layout()));
        (hooks.finish(node), runtime)
    }
}

impl PaintTarget for TextFieldBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl StateHost for TextFieldBuilder {}

impl WidgetLayoutExt for TextFieldBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for TextFieldBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        let focus_trigger = self.focus_trigger;
        let autofocus = self.autofocus;
        let at = self.focus_caret;
        let target = Rc::clone(&self.focus_target);
        let (node, runtime) = self.into_node_builder();
        let id = node.build(cx);
        target.set(Some(id));
        if autofocus {
            focus_field(id, &runtime, at);
        }
        if let Some(trigger) = focus_trigger {
            crate::events::focus::focus_on_trigger(trigger, move || focus_field(id, &runtime, at));
        }
        id
    }
}

fn focus_field(id: crate::kernel::NodeId, runtime: &TextFieldRuntime, at: FocusCaret) {
    crate::events::request_focus(id);
    runtime.input.place_on_focus(at);
}
