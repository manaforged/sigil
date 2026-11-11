use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, TextContent, fragment, show, text};
use crate::layout::{column, row};
use crate::style::{ClassSource, Paint};
use crate::widgets::{
    Orientation, Widget, button, card, checkbox, divider, field, scroll, spacer, text_edit,
    text_field, toggle,
};

use super::parse::{Attrs, Item, Variant};
use super::vocab::Kind;

pub(super) fn build(item: &Rc<Item>) -> Box<dyn IntoNode> {
    let Some(flag) = item.attrs.show else {
        return shown(item);
    };
    let item = Rc::clone(item);
    Box::new(show(flag, move || shown(&item), fragment))
}

fn styled<W: Widget>(mut widget: W, attrs: &Attrs) -> W {
    if let Some(padding) = attrs.padding {
        widget = widget.padding(padding);
    }
    if let Some(layer) = attrs.layer {
        widget = widget.map_node(move |node| node.layer(layer));
    }
    if let Some(class) = attrs.class.clone() {
        widget = widget.map_node(move |node| node.class(ClassSource::Owned(Box::new(class))));
    }
    widget
}

fn stack(mut parent: NodeBuilder, item: &Item) -> NodeBuilder {
    if let Some(gap) = item.attrs.gap {
        parent = parent.gap(gap);
    }
    for child in &item.children {
        parent = parent.child(build(child));
    }
    parent
}

fn label(item: &Item) -> TextContent {
    item.label
        .as_ref()
        .map_or_else(|| TextContent::from(""), TextContent::from)
}

fn shown(item: &Rc<Item>) -> Box<dyn IntoNode> {
    let attrs = &item.attrs;
    match item.kind {
        Kind::Column => Box::new(styled(stack(column(), item), attrs)),
        Kind::Row => Box::new(styled(stack(row(), item), attrs)),
        Kind::Card => Box::new(styled(
            NodeBuilder::from(card(stack(column(), item))),
            attrs,
        )),
        Kind::Scroll => Box::new(styled(
            NodeBuilder::from(scroll(stack(column(), item))),
            attrs,
        )),
        Kind::Field => field_of(item),
        Kind::Text => text_of(item),
        Kind::Button => button_of(item),
        Kind::TextField => text_field_of(item),
        Kind::Checkbox => checkbox_of(item),
        Kind::Toggle => toggle_of(item),
        Kind::TextEdit => text_edit_of(item),
        Kind::Divider => Box::new(styled(divider(Orientation::Horizontal), attrs)),
        Kind::Spacer => Box::new(styled(spacer(), attrs)),
        Kind::Slot => item.slot.as_ref().map_or_else(
            || -> Box<dyn IntoNode> { Box::new(fragment()) },
            |slot| Box::new(styled(slot(), attrs)),
        ),
    }
}

macro_rules! input_of {
    ($item:expr, $widget:path) => {{
        let attrs = &$item.attrs;
        attrs.value.map(|value| {
            let mut node = $widget(value);
            if let Some(placeholder) = &attrs.placeholder {
                node = node.placeholder(TextContent::from(placeholder));
            }
            if let Some(flag) = attrs.disabled {
                node = node.disabled(flag);
            }
            if let Some(action) = attrs.submit.clone() {
                node = node.submit(move |_| action());
            }
            styled(node, attrs)
        })
    }};
}

macro_rules! check_of {
    ($item:expr, $widget:path) => {{
        let attrs = &$item.attrs;
        attrs.checked.map(|checked| {
            let mut node = $widget(checked);
            if let Some(words) = $item.label.as_ref().map(TextContent::from) {
                node = node.label(words);
            }
            if let Some(flag) = attrs.disabled {
                node = node.disabled(flag);
            }
            if let Some(action) = attrs.change.clone() {
                node = node.change(move |_| action());
            }
            styled(node, attrs)
        })
    }};
}

fn boxed(node: Option<impl IntoNode + 'static>) -> Box<dyn IntoNode> {
    node.map_or_else(
        || -> Box<dyn IntoNode> { Box::new(fragment()) },
        |node| Box::new(node),
    )
}

fn field_of(item: &Item) -> Box<dyn IntoNode> {
    let attrs = &item.attrs;
    let child = item
        .children
        .first()
        .filter(|child| child.attrs.show.is_none());
    match child.map(|child| (child.kind, child)) {
        Some((Kind::TextField, child)) => {
            boxed(input_of!(child, text_field).map(|node| styled(field(label(item), node), attrs)))
        }
        Some((Kind::TextEdit, child)) => {
            boxed(input_of!(child, text_edit).map(|node| styled(field(label(item), node), attrs)))
        }
        _ => {
            let control = item
                .children
                .iter()
                .fold(fragment(), |control, child| control.child(build(child)));
            Box::new(styled(field(label(item), control), attrs))
        }
    }
}

fn text_of(item: &Item) -> Box<dyn IntoNode> {
    let mut node = text(label(item));
    if let Some(role) = item.attrs.role {
        node = node.text_role(role);
    }
    Box::new(styled(node, &item.attrs))
}

fn button_of(item: &Item) -> Box<dyn IntoNode> {
    let attrs = &item.attrs;
    let mut node = button(label(item));
    node = match attrs.variant {
        Some(Variant::Primary) => node.primary(),
        Some(Variant::Outline) => node.outline(),
        Some(Variant::Ghost) => node.ghost(),
        Some(Variant::Destructive) => node.destructive(),
        None => node,
    };
    if let Some(flag) = attrs.disabled {
        node = node.disabled(flag);
    }
    if let Some(action) = attrs.activate.clone() {
        node = node.activate(move || action());
    }
    Box::new(styled(node, attrs))
}

fn text_field_of(item: &Item) -> Box<dyn IntoNode> {
    boxed(input_of!(item, text_field))
}

fn text_edit_of(item: &Item) -> Box<dyn IntoNode> {
    boxed(input_of!(item, text_edit))
}

fn checkbox_of(item: &Item) -> Box<dyn IntoNode> {
    boxed(check_of!(item, checkbox))
}

fn toggle_of(item: &Item) -> Box<dyn IntoNode> {
    boxed(check_of!(item, toggle))
}
