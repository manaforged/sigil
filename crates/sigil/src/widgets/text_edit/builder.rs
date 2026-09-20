use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use crate::layout::prelude::LayoutBuilderExt;
use crate::reactive::{Signal, signal};
use crate::style::{ClassSource, NodeStyleExt, StateHost, Style, StyleAccess};

use crate::widgets::interaction::InteractionState;

use super::super::clipboard::{Clipboard, clipboard};
use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{BuilderMutator, disabled_now};
use super::handlers::{TextEditRuntime, TextEditState, attach_key_handlers, noop_change};
use super::ime::attach_ime_handlers;
use super::render::{build_base_node, content_padding, edit_layout};

pub(super) type TextChange = Rc<dyn Fn(&str) + 'static>;

pub struct TextEditBuilder {
    pub(super) value: Signal<String>,
    pub(super) placeholder: TextContent,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) change: Option<TextChange>,
    pub(super) submit: Option<TextChange>,
    pub(super) clipboard: Option<Rc<dyn Clipboard>>,
    pub(super) caret_visible: Option<Signal<bool>>,
    pub(super) caret_position: Option<Signal<usize>>,
    pub(super) content_padding: (f32, f32),
    state: Option<TextEditState>,
    pub(super) style_accumulator: Style,
    mutators: Vec<BuilderMutator>,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn text_edit(value: Signal<String>) -> TextEditBuilder {
    TextEditBuilder::new(value)
}

impl TextEditBuilder {
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
            caret_position: None,
            content_padding: content_padding(),
            state: None,
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

    #[must_use]
    pub const fn caret_position(mut self, position: Signal<usize>) -> Self {
        self.caret_position = Some(position);
        self
    }

    #[must_use]
    pub const fn content_padding(mut self, horizontal: f32, vertical: f32) -> Self {
        self.content_padding = (horizontal, vertical);
        self
    }

    #[must_use]
    pub fn state(mut self, state: TextEditState) -> Self {
        self.value = state.value;
        self.caret_position = Some(state.caret);
        self.state = Some(state);
        self
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
        let state = self.state.unwrap_or_else(|| {
            TextEditState::new(
                value,
                self.caret_position
                    .unwrap_or_else(|| signal(value.peek().len())),
            )
        });
        let runtime = TextEditRuntime::new(
            state,
            disabled,
            change,
            submit,
            clipboard,
            caret_visible,
            self.content_padding,
        );

        let user_overlay = style_accumulator.take_state_overlay();
        let node = build_base_node(&runtime, &placeholder, interaction, user_overlay);
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
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}
