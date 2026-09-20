use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use crate::layout::prelude::LayoutBuilderExt;
use crate::reactive::{Signal, derived, signal};
use crate::style::{ClassSource, NodeStyleExt, StateHost, Style, StyleAccess};

use crate::widgets::interaction::InteractionState;

use super::super::clipboard::{Clipboard, clipboard};
use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{BuilderMutator, disabled_now};
use super::handlers::{TextFieldRuntime, attach_key_handlers, noop_change};
use super::ime::attach_ime_handlers;
use super::render::{FieldInputs, build_base_node, field_layout};

pub(super) type TextChange = Rc<dyn Fn(&str) + 'static>;

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
    pub(super) password: bool,
    pub(super) reveal: Option<Signal<bool>>,
    pub(super) style_accumulator: Style,
    mutators: Vec<BuilderMutator>,
    layout_overrides: LayoutOverrides,
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
            clipboard: None,
            caret_visible: None,
            scroll_offset: None,
            focus_trigger: None,
            password: false,
            reveal: None,
            style_accumulator: Style::new(),
            mutators: Vec::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
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
    pub const fn disabled(mut self, flag: Signal<bool>) -> Self {
        self.disabled = Some(flag);
        self
    }

    #[must_use]
    pub const fn invalid_when(mut self, invalid: Signal<bool>) -> Self {
        self.invalid = Some(invalid);
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
    pub const fn password(mut self) -> Self {
        self.password = true;
        self
    }

    #[must_use]
    pub const fn reveal(mut self, show: Signal<bool>) -> Self {
        self.reveal = Some(show);
        self
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one assembly path: inputs, runtime, base node, mutators"
    )]
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

        let width = layout_overrides.width_hint();
        let height = layout_overrides.height_hint();
        let column = super::selection::column_width(width);
        let reveal = self.reveal.unwrap_or_else(|| signal(false));
        let runtime = TextFieldRuntime::new(
            value,
            disabled,
            change,
            submit,
            clipboard,
            caret_visible,
            scroll,
            column,
            self.password,
            reveal,
        );
        let clip_fail = runtime.clip_fail;
        let invalid = Some(derived(move || {
            invalid.is_some_and(|flag| flag.get()) || clip_fail.get()
        }));

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
        let node = attach_ime_handlers(node, &runtime);
        let mut node = mutators.into_iter().fold(node, |node, mutate| mutate(node));
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
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        let focus_trigger = self.focus_trigger;
        let (node, _runtime) = self.into_node_builder();
        let id = node.build(cx);
        if let Some(trigger) = focus_trigger {
            if trigger.peek() > 0 {
                crate::events::request_focus(id);
            }
            install_focus_trigger(id, trigger);
        }
        id
    }
}

fn install_focus_trigger(id: crate::kernel::NodeId, trigger: Signal<u64>) {
    let last = std::cell::Cell::new(trigger.peek());
    crate::reactive::effect_named("sigil::text_field::focus_when", move || {
        let generation = trigger.get();
        if generation != last.replace(generation) {
            crate::events::request_focus(id);
        }
    });
}
