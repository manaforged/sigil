use crate::kernel::{AccessibleName, Cx, IntoNode, NodeId, Role, TextContent, container, text};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Reactive, Signal, derived};
use crate::style::{DisplayKind, StyleAtom, Token, space};

use super::color_field::ColorFieldBuilder;
use super::number_field::{NumberFieldBuilder, NumericKind};
use super::text_field::TextFieldBuilder;

const FIELD_GAP: f32 = space(1);
static ERROR_VISIBLE: &[StyleAtom] = &[];
static ERROR_HIDDEN: &[StyleAtom] = &[StyleAtom::Display(DisplayKind::None)];

pub struct FieldBuilder<C> {
    label: TextContent,
    control: C,
    error: Option<Signal<Option<String>>>,
}

#[must_use]
pub fn field<C: IntoNode + 'static>(label: impl Into<TextContent>, control: C) -> FieldBuilder<C> {
    FieldBuilder {
        label: label.into(),
        control,
        error: None,
    }
}

pub trait FieldControl: IntoNode + Sized + 'static {
    #[must_use]
    fn invalid_when(self, invalid: impl Reactive<bool>) -> Self;
}

impl FieldControl for TextFieldBuilder {
    fn invalid_when(self, invalid: impl Reactive<bool>) -> Self {
        Self::invalid_when(self, invalid)
    }
}

impl<T: NumericKind + 'static> FieldControl for NumberFieldBuilder<T> {
    fn invalid_when(self, invalid: impl Reactive<bool>) -> Self {
        Self::invalid_when(self, invalid)
    }
}

impl FieldControl for ColorFieldBuilder {
    fn invalid_when(self, invalid: impl Reactive<bool>) -> Self {
        Self::invalid_when(self, invalid)
    }
}

impl<C: FieldControl> FieldBuilder<C> {
    #[must_use]
    pub fn error(mut self, error: impl Reactive<Option<String>>) -> Self {
        let error = error.into_signal();
        self.control = self
            .control
            .invalid_when(derived(move || error.get().is_some()));
        self.error = Some(error);
        self
    }
}

impl<C: IntoNode + 'static> IntoNode for FieldBuilder<C> {
    fn build(self, cx: &mut Cx) -> NodeId {
        let invalid = self
            .error
            .map(|error| derived(move || error.get().is_some()));
        let mut root = container()
            .role(Role::GenericContainer)
            .layout(LayoutStyle::flex_column().fill_width().gap(FIELD_GAP))
            .child(text(self.label).nowrap())
            .child(self.control)
            .on_added(move |tree, id| {
                let Some(node) = tree.get(id) else { return };
                let [label_id, control_id, ..] = *node.children() else {
                    return;
                };
                let error_id = node.children().get(2).copied();
                if let Some(target) = tree.get_mut(control_id) {
                    target.a11y.labelled_by = Some(label_id);
                    target.a11y.name = AccessibleName::None;
                    target.a11y.described_by = error_id;
                    target.a11y.invalid = invalid;
                }
            });
        if let Some(error) = self.error {
            root = root.child(error_line(error));
        }
        cx.add(root)
    }
}

fn error_line(error: Signal<Option<String>>) -> crate::kernel::TextBuilder {
    text(move || error.get().unwrap_or_default())
        .role(Role::Label)
        .class("text-xs font-medium")
        .class(Token::Error12.text())
        .style_reactive(move || {
            if error.get().is_some() {
                ERROR_VISIBLE
            } else {
                ERROR_HIDDEN
            }
        })
}
