use std::cell::RefCell;
use std::rc::Rc;

use sigil_core::{Cx, IntoNode, NodeBuilder, Role, TextBuilder, TextContent, container, text};
use sigil_events::prelude::{Key, KeyEvent, NodeEventsExt, PointerEvent};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle, LengthPercentage};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{NodeStyleExt, StateStyle, StyleAtom, StyleVariant};

use crate::interaction::InteractionState;

use super::shared::{binary_style_state, disabled_now};
use super::text_edit::TextEdit;

type TextChange = Rc<dyn Fn(&str) + 'static>;

#[derive(Clone)]
struct TextFieldRuntime {
    value: Signal<String>,
    disabled: Option<Signal<bool>>,
    edit: Rc<RefCell<TextEdit>>,
    on_change: TextChange,
    on_submit: TextChange,
}

/// Builder returned by [`text_field`].
pub struct TextFieldBuilder {
    value: Signal<String>,
    placeholder: TextContent,
    disabled: Option<Signal<bool>>,
    on_change: Option<TextChange>,
    on_submit: Option<TextChange>,
}

static FIELD_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];
static FIELD_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];
static FIELD_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static FIELD_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
static FIELD_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("on-surface"),
    StyleAtom::Opacity(8_800),
];
static FIELD_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("border"),
    StyleAtom::Opacity(5_000),
];

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
        }
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

    fn into_node_builder(self) -> NodeBuilder {
        let enabled = !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let node = self.base_node(interaction);
        let runtime = TextFieldRuntime {
            value: self.value,
            disabled: self.disabled,
            edit: Rc::new(RefCell::new(TextEdit::new(self.value.peek()))),
            on_change: self.on_change.unwrap_or_else(noop_text_change),
            on_submit: self.on_submit.unwrap_or_else(noop_text_change),
        };
        interaction.install(text_field_key_handlers(node, runtime))
    }

    fn base_node(&self, interaction: InteractionState) -> NodeBuilder {
        let disabled = self.disabled;
        let placeholder = text_content_untracked(&self.placeholder);
        let displayed = displayed_text(
            self.value,
            interaction.focus_visible(),
            self.placeholder.clone(),
        );
        container()
            .role(Role::TextInput)
            .aria_label(accessible_label(placeholder.as_str()))
            .aria_text_value_signal(self.value)
            .aria_placeholder(placeholder)
            .layout(text_field_layout())
            .class("rounded-md")
            .style_reactive(move || {
                text_field_variant().state(text_field_state(interaction, disabled))
            })
            .on_pointer_up(move |event| text_field_pointer_up(event, interaction))
            .child(text_field_inner(displayed))
    }
}

impl IntoNode for TextFieldBuilder {
    fn build(self, cx: &mut Cx) -> sigil_core::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn text_field_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .width_px(180.0)
        .align_items(AlignItems::Center)
        .with_taffy(|style| {
            style.padding.left = LengthPercentage::length(1.0);
            style.padding.right = LengthPercentage::length(1.0);
            style.padding.top = LengthPercentage::length(1.0);
            style.padding.bottom = LengthPercentage::length(1.0);
        })
}

fn text_field_inner(displayed: Signal<String>) -> NodeBuilder {
    container()
        .layout(text_field_inner_layout())
        .class("rounded-sm")
        .class("bg-surface")
        .child(text_field_text(displayed))
}

fn text_field_inner_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .width_px(178.0)
        .align_items(AlignItems::Center)
        .with_taffy(|style| {
            style.padding.left = LengthPercentage::length(12.0);
            style.padding.right = LengthPercentage::length(12.0);
            style.padding.top = LengthPercentage::length(8.0);
            style.padding.bottom = LengthPercentage::length(8.0);
        })
}

fn text_field_text(displayed: Signal<String>) -> TextBuilder {
    text(displayed)
        .layout(LayoutStyle::block().width_px(156.0))
        .class("text-on-surface")
}

fn text_field_key_handlers(node: NodeBuilder, runtime: TextFieldRuntime) -> NodeBuilder {
    let any_key = runtime.clone();
    let backspace = runtime.clone();
    let delete = runtime.clone();
    let left = runtime.clone();
    let right = runtime.clone();
    let home = runtime.clone();
    let end = runtime.clone();
    let submit = runtime;
    node.on_key_any(move |event| insert_text(event, &any_key))
        .on_key(Key::Backspace, move |event| {
            edit_value(event, &backspace, TextEdit::backspace);
        })
        .on_key(Key::Delete, move |event| {
            edit_value(event, &delete, TextEdit::delete_forward);
        })
        .on_key(Key::ArrowLeft, move |event| {
            move_caret(event, &left, TextEdit::move_left);
        })
        .on_key(Key::ArrowRight, move |event| {
            move_caret(event, &right, TextEdit::move_right);
        })
        .on_key(Key::Home, move |event| {
            move_caret(event, &home, TextEdit::move_home);
        })
        .on_key(Key::End, move |event| {
            move_caret(event, &end, TextEdit::move_end);
        })
        .on_key(Key::Enter, move |event| submit_text(event, &submit))
}

fn insert_text(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    let Some(text) = event
        .text
        .as_deref()
        .filter(|text| is_single_line_scalar(text))
    else {
        return;
    };
    let mut edit = runtime.edit.borrow_mut();
    edit.insert_str(text);
    write_value(runtime.value, edit.as_str());
    (runtime.on_change)(edit.as_str());
    event.stop_propagation();
}

fn edit_value(event: &mut KeyEvent, runtime: &TextFieldRuntime, edit: impl FnOnce(&mut TextEdit)) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit.borrow_mut();
    let before = (state.len(), state.is_empty());
    edit(&mut state);
    if (state.len(), state.is_empty()) != before {
        write_value(runtime.value, state.as_str());
        (runtime.on_change)(state.as_str());
    }
}

fn move_caret(event: &mut KeyEvent, runtime: &TextFieldRuntime, edit: impl FnOnce(&mut TextEdit)) {
    event.stop_propagation();
    if !runtime.disabled() {
        let mut state = runtime.edit.borrow_mut();
        edit(&mut state);
        debug_assert!(state.caret() <= state.len());
    }
}

fn submit_text(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let value = runtime.value.peek();
    (runtime.on_submit)(value.as_str());
}

fn write_value(value: Signal<String>, next: &str) {
    value.update(|current| {
        current.clear();
        current.push_str(next);
    });
}

fn text_field_pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    let _ = interaction.release_pointer(event);
}

fn displayed_text(
    value: Signal<String>,
    focused: Signal<bool>,
    placeholder: TextContent,
) -> Signal<String> {
    derived_eq(move || {
        let current = value.get();
        if current.is_empty() && !focused.get() {
            text_content(&placeholder)
        } else {
            current
        }
    })
}

fn text_content(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.clone(),
        TextContent::Reactive(value) => value.get(),
    }
}

fn text_content_untracked(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.clone(),
        TextContent::Reactive(value) => value.peek(),
    }
}

fn accessible_label(placeholder: &str) -> String {
    (!placeholder.is_empty())
        .then_some(placeholder)
        .filter(|text| !text.is_empty())
        .map_or_else(|| String::from("Text field"), ToOwned::to_owned)
}

fn is_single_line_scalar(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(ch) = chars.next() else {
        return false;
    };
    chars.next().is_none() && ch != '\n' && ch != '\r'
}

impl TextFieldRuntime {
    fn disabled(&self) -> bool {
        disabled_now(self.disabled)
    }
}

fn noop_text_change() -> TextChange {
    Rc::new(|_| {})
}

fn text_field_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> sigil_style::StyleState {
    binary_style_state(interaction, disabled)
}

const fn text_field_variant() -> StyleVariant {
    StyleVariant::new(
        FIELD_BASE,
        StateStyle::new(
            FIELD_REST,
            FIELD_FOCUS,
            FIELD_HOVER,
            FIELD_DOWN,
            FIELD_DISABLED,
        ),
    )
}
