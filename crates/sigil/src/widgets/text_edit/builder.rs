use crate::reactive::Reactive;
use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{Cx, IntoNode, NodeBuilder, TextContent};
use crate::reactive::{Signal, signal};
use crate::style::{PaintTarget, StateHost, Style, StyleSlot};

use crate::widgets::interaction::InteractionState;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::{BuilderMutator, disabled_now};
use super::handlers::{TextEditRuntime, TextEditState, attach_key_handlers, noop_change};
use super::ime::attach_ime_handlers;
use super::render::{build_base_node, content_padding, edit_layout};
use crate::clipboard::{Clipboard, clipboard};

pub(super) type TextChange = Rc<dyn Fn(&str) + 'static>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubmitKey {
    #[default]
    CtrlEnter,
    Enter,
}

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
    autofocus: bool,
    focus_trigger: Option<Signal<u64>>,
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
            submit_key: SubmitKey::CtrlEnter,
            clipboard: None,
            caret_visible: None,
            caret_position: None,
            caret_rect: None,
            content_padding: content_padding(),
            chrome: true,
            state: None,
            autofocus: false,
            focus_trigger: None,
            style_accumulator: Style::new(),
            mutators: Vec::new(),
            layout_overrides: LayoutOverrides::new(),
        }
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
    pub const fn caret_rect(mut self, rect: Signal<Option<Rect>>) -> Self {
        self.caret_rect = Some(rect);
        self
    }

    #[must_use]
    pub const fn content_padding(mut self, horizontal: f32, vertical: f32) -> Self {
        self.content_padding = (horizontal, vertical);
        self
    }

    #[must_use]
    pub const fn chrome(mut self, enabled: bool) -> Self {
        self.chrome = enabled;
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
    pub fn disabled(mut self, flag: impl Reactive<bool>) -> Self {
        self.disabled = Some(flag.into_signal());
        self
    }

    #[must_use]
    pub fn change(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.change = Some(Rc::new(f));
        self
    }

    #[must_use]
    pub const fn submit_on(mut self, key: SubmitKey) -> Self {
        self.submit_key = key;
        self
    }

    #[must_use]
    pub fn submit(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.submit = Some(Rc::new(f));
        self
    }

    #[must_use]
    pub const fn autofocus(mut self) -> Self {
        self.autofocus = true;
        self
    }

    #[must_use]
    pub const fn focus_when(mut self, trigger: Signal<u64>) -> Self {
        self.focus_trigger = Some(trigger);
        self
    }

    #[must_use]
    pub fn clipboard(mut self, clipboard: impl Clipboard + 'static) -> Self {
        self.clipboard = Some(Rc::new(clipboard));
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let value = self.value;
        let interaction = InteractionState::new(!disabled_now(self.disabled));
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
        let mut node = self
            .mutators
            .into_iter()
            .fold(node, |node, mutate| mutate(node));
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node.with(self.layout_overrides.apply(edit_layout()))
    }
}

impl PaintTarget for TextEditBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
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
        let (autofocus, trigger) = (self.autofocus, self.focus_trigger);
        let node = self
            .layout_overrides
            .hooks()
            .finish(self.into_node_builder());
        let node = if autofocus {
            <NodeBuilder as crate::events::NodeEventsExt>::autofocus(node)
        } else {
            node
        };
        let id = node.build(cx);
        if let Some(trigger) = trigger {
            crate::events::focus::focus_on_trigger(trigger, move || {
                crate::events::request_focus(id);
            });
        }
        id
    }
}
