use crate::kernel::{
    AccessibleName, Cx, IntoNode, NodeBuilder, NodeId, Role, TextContent, container, node, text,
};
use crate::layout::Display;
use crate::layout::LayoutStyle;
use crate::layout::Space;
use crate::reactive::{Reactive, Signal, memo};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_text;
use crate::style::{Paint, StateStyleBuilder, States, StyleAtom, Tone, space};
use crate::widgets::Widget;

use super::text_field::TextFieldBuilder;

mod parsed;

pub(crate) use parsed::{FieldParser, ParsedField, parsed_controls};

pub(crate) const FIELD_GAP: f32 = space(1);
fn shown(visible: bool) -> &'static [StyleAtom] {
    static HIDDEN: &[StyleAtom] = &[StyleAtom::Display(Display::None)];
    if visible { &[] } else { HIDDEN }
}

/// A labelled control; layout and `map_node` change the field root, and style, state, and focus setters change the control.
#[derive(Debug)]
#[must_use]
pub struct FieldBuilder<C> {
    label: TextContent,
    control: C,
    error: Option<Signal<Option<String>>>,
    required: Option<Signal<bool>>,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

/// A label above a control.
pub fn field<C: IntoNode + 'static>(label: impl Into<TextContent>, control: C) -> FieldBuilder<C> {
    FieldBuilder {
        label: label.into(),
        control,
        error: None,
        required: None,
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

pub(crate) mod field_sealed {
    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed {}
}

/// A control that can show an invalid state: `text_field`, `number_field`, `color_field`, `date_field`, `time_field`.
pub trait FieldControl: IntoNode + Sized + field_sealed::Sealed + 'static {
    /// Marks the control invalid while the value is true.
    #[must_use]
    fn invalid(self, invalid: impl Reactive<bool>) -> Self;
}

impl field_sealed::Sealed for TextFieldBuilder {}

impl FieldControl for TextFieldBuilder {
    fn invalid(self, invalid: impl Reactive<bool>) -> Self {
        Self::invalid(self, invalid)
    }
}

impl<C: FieldControl> FieldBuilder<C> {
    /// Shows the error text below the control and marks the control invalid while it is `Some`.
    pub fn error(mut self, error: impl Reactive<Option<String>>) -> Self {
        let error = error.into_signal();
        self.control = self.control.invalid(memo(move || error.get().is_some()));
        self.error = Some(error);
        self
    }
}

impl<C> FieldBuilder<C> {
    /// Shows the required mark after the label and reports the control as required while true.
    pub fn required(mut self, required: impl Reactive<bool>) -> Self {
        self.required = Some(required.into_signal());
        self
    }
}

impl<C: IntoNode + 'static> IntoNode for FieldBuilder<C> {
    fn build(self, cx: &mut Cx) -> NodeId {
        let required = self.required;
        let mut root = container()
            .role(Role::GenericContainer)
            .layout(
                LayoutStyle::flex_column()
                    .fill_width()
                    .gap(Space::px(FIELD_GAP)),
            )
            .child(label_row(self.label, required))
            .child(self.control)
            .added(move |tree, id| {
                let Some(node) = tree.get(id) else { return };
                let [row_id, control_id, ..] = *node.children() else {
                    return;
                };
                let error_id = node.children().get(2).copied();
                let label_id = if required.is_some() {
                    tree.get(row_id)
                        .and_then(|row| row.children().first().copied())
                } else {
                    Some(row_id)
                };
                let Some(label_id) = label_id else {
                    return;
                };
                let target_id = resolve_control_target(tree, control_id);
                if let Some(target) = tree.get_mut(target_id) {
                    target.a11y.labelled_by = Some(label_id);
                    target.a11y.name = AccessibleName::None;
                    target.a11y.described_by = error_id;
                    target.a11y.required = required;
                }
            });
        if let Some(error) = self.error {
            root = root.child(error_line(error));
        }
        cx.add(self.layout_overrides.finish(root))
    }
}

fn find_control_descendant(tree: &Cx, id: NodeId) -> Option<NodeId> {
    let node = tree.get(id)?;
    if node.a11y.role != Role::GenericContainer {
        return Some(id);
    }
    node.children()
        .iter()
        .copied()
        .find_map(|child| find_control_descendant(tree, child))
}

fn resolve_control_target(tree: &Cx, control_id: NodeId) -> NodeId {
    find_control_descendant(tree, control_id).unwrap_or(control_id)
}

impl<C: Paint> Paint for FieldBuilder<C> {
    fn paint_mut(&mut self) -> &mut StateStyleBuilder {
        self.control.paint_mut()
    }
}

impl<C: States> States for FieldBuilder<C> {
    crate::style::state_methods!(forward control);
}

impl<C: Widget> Widget for FieldBuilder<C> {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.layout_overrides.node_hook(f);
        self
    }

    crate::widgets::widget_layout_methods!(@edit layout_overrides.edit);
    crate::widgets::forward_control_methods!(direct control);
}

fn label_row(label: TextContent, required: Option<Signal<bool>>) -> Box<dyn IntoNode> {
    let label = text(label).nowrap();
    match required {
        Some(required) => node(
            container()
                .role(Role::GenericContainer)
                .layout(LayoutStyle::flex_row().gap(Space::px(FIELD_GAP)))
                .child(label)
                .child(
                    text(theme_text(StringKey::RequiredMark))
                        .hittable(false)
                        .class(Tone::Error.step(11).text())
                        .style_reactive(move || shown(required.get())),
                ),
        ),
        None => node(label),
    }
}

fn error_line(error: Signal<Option<String>>) -> crate::kernel::TextBuilder {
    text(move || error.get().unwrap_or_default())
        .role(Role::Label)
        .class("text-xs font-medium")
        .class(Tone::Error.step(12).text())
        .style_reactive(move || shown(error.get().is_some()))
}
