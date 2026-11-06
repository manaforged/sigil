use std::cell::Cell;
use std::rc::Rc;

use serde_json::Value;

use crate::kernel::{NodeBuilder, TextContent, container, text};
use crate::layout::{AlignItems, LayoutStyle};
use crate::style::{Token, space};
use crate::widgets::{
    Widget, button, card, divider_horizontal, divider_vertical, markdown_inline, tabs,
};

use super::document::{MAX_DEPTH, Surface};
use super::model::{Model, display};
use super::spec::{Axis, ButtonLook, Children, Dynamic, Event, Flex, Kind, TextStyle};
use super::{glyph, media};

const MAX_RENDERED: usize = 2048;
const MAX_TEMPLATE_ITEMS: usize = 256;

pub(super) type Handler = Rc<dyn Fn(&str, &str)>;

#[derive(Clone)]
pub(super) struct Render {
    pub(super) surface: Rc<Surface>,
    pub(super) model: Model,
    pub(super) handler: Option<Handler>,
    pub(super) scope: Rc<str>,
    pub(super) depth: usize,
    pub(super) budget: Rc<Cell<usize>>,
}

pub(super) fn note(message: impl Into<String>) -> NodeBuilder {
    text(message.into())
        .wrap()
        .class("text-xs")
        .class(Token::OnSurfaceMuted.text())
        .into()
}

impl Render {
    pub(super) fn new(surface: Surface, handler: Option<Handler>) -> Self {
        let model = Model::new(surface.data.clone());
        Self {
            surface: Rc::new(surface),
            model,
            handler,
            scope: Rc::from(""),
            depth: 0,
            budget: Rc::new(Cell::new(MAX_RENDERED)),
        }
    }

    pub(super) fn node(&self, id: &str) -> NodeBuilder {
        let left = self.budget.get();
        if self.depth > MAX_DEPTH || left == 0 {
            return note("UI surface is too large to show in full.");
        }
        self.budget.set(left - 1);
        let Some(component) = self.surface.components.get(id) else {
            return note(format!("Missing component \"{id}\"."));
        };
        let inner = Self {
            depth: self.depth + 1,
            ..self.clone()
        };
        let node = inner.kind(&component.kind);
        match component.weight {
            Some(weight) if weight > 0.0 => node.grow(weight),
            _ => node,
        }
    }

    pub(super) fn path(&self, raw: &str) -> String {
        if raw.starts_with('/') {
            raw.to_string()
        } else {
            format!("{}/{raw}", self.scope)
        }
    }

    pub(super) fn bound(&self, value: &Dynamic) -> Option<String> {
        match value {
            Dynamic::Path(raw) => Some(self.path(raw)),
            Dynamic::Literal(_) => None,
        }
    }

    pub(super) fn value(&self, value: &Dynamic) -> Value {
        match value {
            Dynamic::Literal(literal) => literal.clone(),
            Dynamic::Path(raw) => self.model.peek(&self.path(raw)),
        }
    }

    pub(super) fn live(&self, value: &Dynamic) -> TextContent {
        match self.bound(value) {
            Some(path) => {
                let model = self.model;
                TextContent::from(move || display(&model.get(&path)))
            }
            None => TextContent::from(display(&self.value(value))),
        }
    }

    fn kind(&self, kind: &Kind) -> NodeBuilder {
        match kind {
            Kind::Text { text, style } => self.text(text, *style),
            Kind::Image {
                url,
                alt,
                size,
                fit,
            } => media::picture(
                &display(&self.value(url)),
                display(&self.value(alt)),
                *size,
                *fit,
            ),
            Kind::Icon { glyph } => glyph::draw(*glyph).into(),
            Kind::Row { children, flex } => self.flex(LayoutStyle::flex_row(), children, *flex),
            Kind::Column { children, flex } => {
                self.flex(LayoutStyle::flex_column(), children, *flex)
            }
            Kind::List {
                children,
                axis,
                align,
            } => self.list(children, *axis, *align),
            Kind::Card { child } => card(self.node(child)).into(),
            Kind::Tabs { tabs } => self.tabs(tabs),
            Kind::Divider {
                axis: Axis::Horizontal,
            } => divider_horizontal(),
            Kind::Divider {
                axis: Axis::Vertical,
            } => divider_vertical(),
            Kind::Button { child, look, event } => self.button(child, *look, event.as_ref()),
            Kind::Unsupported { name } => note(format!("Unsupported component \"{name}\".")),
            input => self.input(input),
        }
    }

    fn text(&self, value: &Dynamic, style: TextStyle) -> NodeBuilder {
        let (size, tone) = match style {
            TextStyle::H1 => ("text-xl font-bold", Token::OnSurface),
            TextStyle::H2 => ("text-lg font-bold", Token::OnSurface),
            TextStyle::H3 => ("text-base font-bold", Token::OnSurface),
            TextStyle::H4 => ("text-sm font-bold", Token::OnSurface),
            TextStyle::H5 => ("text-sm font-medium", Token::OnSurfaceMuted),
            TextStyle::Body => ("text-sm", Token::OnSurface),
            TextStyle::Caption => ("text-xs", Token::OnSurfaceMuted),
        };
        let node = match self.bound(value) {
            Some(_) => text(self.live(value)),
            None => {
                let source = display(&self.value(value));
                markdown_inline(source.trim_start_matches('#').trim_start())
            }
        };
        node.wrap().class(size).class(tone.text()).into()
    }

    pub(super) fn children(&self, children: &Children) -> Vec<NodeBuilder> {
        match children {
            Children::Ids(ids) => ids.iter().map(|id| self.node(id)).collect(),
            Children::Template { id, path } => {
                let path = self.path(path);
                let count = match self.model.peek(&path) {
                    Value::Array(items) => items.len().min(MAX_TEMPLATE_ITEMS),
                    _ => 0,
                };
                (0..count)
                    .map(|index| {
                        let scope = Rc::from(format!("{path}/{index}"));
                        Self {
                            scope,
                            ..self.clone()
                        }
                        .node(id)
                    })
                    .collect()
            }
        }
    }

    fn flex(&self, layout: LayoutStyle, children: &Children, flex: Flex) -> NodeBuilder {
        let layout = layout
            .gap(space(2))
            .min_width(0.0)
            .justify_content(flex.justify)
            .align_items(flex.align);
        self.children(children)
            .into_iter()
            .fold(container().layout(layout), NodeBuilder::child)
    }

    fn list(&self, children: &Children, axis: Axis, align: AlignItems) -> NodeBuilder {
        let layout = match axis {
            Axis::Horizontal => LayoutStyle::flex_row(),
            Axis::Vertical => LayoutStyle::flex_column(),
        };
        let flex = Flex {
            justify: crate::layout::JustifyContent::Start,
            align,
        };
        self.flex(layout, children, flex)
    }

    fn tabs(&self, panes: &[(Dynamic, String)]) -> NodeBuilder {
        let mut strip = tabs();
        for (title, child) in panes {
            let (render, child) = (self.clone(), child.clone());
            strip = strip.tab(display(&self.value(title)), move || render.node(&child));
        }
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fill_width()
                    .min_height(space(30)),
            )
            .child(strip.into_container())
    }

    fn button(&self, child: &str, look: ButtonLook, event: Option<&Event>) -> NodeBuilder {
        let label = match self.surface.components.get(child).map(|c| &c.kind) {
            Some(Kind::Text { text, .. }) => self.live(text),
            _ => TextContent::from(child.to_string()),
        };
        let mut control = button(label);
        control = match look {
            ButtonLook::Default => control,
            ButtonLook::Primary => control.primary(),
            ButtonLook::Borderless => control.ghost(),
        };
        if let (Some(event), Some(handler)) = (event, self.handler.clone()) {
            let name = event.name.clone();
            let context: Vec<(String, Dynamic)> = event.context.clone();
            let render = self.clone();
            control = control.activate(move || {
                let mut map = serde_json::Map::new();
                for (key, value) in &context {
                    map.insert(key.clone(), render.value(value));
                }
                handler(&name, &Value::Object(map).to_string());
            });
        }
        control.into()
    }
}
