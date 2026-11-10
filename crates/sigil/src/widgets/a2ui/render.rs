use crate::layout::Space;
use std::cell::Cell;
use std::rc::Rc;

use serde_json::Value;

use crate::kernel::{NodeBuilder, TextContent, container, fragment, text};
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::{Signal, derived, signal, untrack};
use crate::style::{Token, space};
use crate::widgets::{Layout, Orientation, Widget, button, card, divider, tabs};

use super::document::{MAX_BYTES, Surface};
use super::model::{Model, display};
use super::spec::{Axis, ButtonLook, Children, Dynamic, Event, Flex, Kind, TextStyle};
use super::{glyph, media, prose};

const MAX_RENDERED: usize = 1024;
const MAX_RENDER_DEPTH: usize = 16;
const MAX_TEXT_BYTES: usize = MAX_BYTES;
const MAX_TEMPLATE_ITEMS: usize = 256;
const TOO_LARGE: &str = "UI surface is too large to show in full.";

pub(super) struct Budget {
    nodes: Cell<usize>,
    bytes: Cell<usize>,
    noted: Cell<bool>,
}

impl Budget {
    const fn new() -> Self {
        Self {
            nodes: Cell::new(MAX_RENDERED),
            bytes: Cell::new(MAX_TEXT_BYTES),
            noted: Cell::new(false),
        }
    }

    fn take(&self) -> bool {
        let nodes = self.nodes.get();
        let open = nodes > 0 && self.bytes.get() > 0;
        if open {
            self.nodes.set(nodes - 1);
        }
        open
    }

    pub(super) fn spend(&self, bytes: usize) {
        self.bytes.set(self.bytes.get().saturating_sub(bytes));
    }

    fn refuse(&self) -> NodeBuilder {
        if self.noted.replace(true) {
            fragment()
        } else {
            note(TOO_LARGE)
        }
    }
}

pub(super) struct Ancestor {
    id: String,
    scope: Rc<str>,
    parent: Option<Rc<Self>>,
}

fn contains(chain: Option<&Rc<Ancestor>>, id: &str, scope: &str) -> bool {
    let mut link = chain;
    while let Some(ancestor) = link {
        if ancestor.id == id && &*ancestor.scope == scope {
            return true;
        }
        link = ancestor.parent.as_ref();
    }
    false
}

pub(super) type Handler = Rc<dyn Fn(&str, &str)>;

#[derive(Clone)]
pub(super) struct Render {
    pub(super) surface: Rc<Surface>,
    pub(super) model: Model,
    pub(super) handler: Option<Handler>,
    pub(super) scope: Rc<str>,
    pub(super) depth: usize,
    pub(super) budget: Rc<Budget>,
    pub(super) chain: Option<Rc<Ancestor>>,
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
            budget: Rc::new(Budget::new()),
            chain: None,
        }
    }

    fn mark(&self, message: String) -> NodeBuilder {
        self.budget.spend(message.len());
        note(message)
    }

    pub(super) fn node(&self, id: &str) -> NodeBuilder {
        if self.depth > MAX_RENDER_DEPTH || !self.budget.take() {
            return self.budget.refuse();
        }
        if contains(self.chain.as_ref(), id, &self.scope) {
            return self.mark(format!("Component \"{id}\" contains itself."));
        }
        let Some(component) = self.surface.components.get(id) else {
            return self.mark(format!("Missing component \"{id}\"."));
        };
        let chain = Ancestor {
            id: id.to_string(),
            scope: Rc::clone(&self.scope),
            parent: self.chain.clone(),
        };
        let inner = Self {
            depth: self.depth + 1,
            chain: Some(Rc::new(chain)),
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

    pub(super) fn tracked(&self, value: &Dynamic) -> Value {
        match value {
            Dynamic::Literal(literal) => literal.clone(),
            Dynamic::Path(raw) => self.model.get(&self.path(raw)),
        }
    }

    pub(super) fn source(&self, value: &Dynamic) -> Signal<String> {
        let model = self.model;
        let first = self.shown(value);
        self.bound(value).map_or_else(
            || signal(first),
            |path| derived(move || display(&model.get(&path))),
        )
    }

    pub(super) fn shown(&self, value: &Dynamic) -> String {
        let shown = display(&self.value(value));
        self.budget.spend(shown.len());
        shown
    }

    pub(super) fn live(&self, value: &Dynamic) -> TextContent {
        let model = self.model;
        let first = self.shown(value);
        self.bound(value).map_or_else(
            || TextContent::from(first),
            |path| TextContent::from(move || display(&model.get(&path))),
        )
    }

    fn kind(&self, kind: &Kind) -> NodeBuilder {
        match kind {
            Kind::Text { text, style } => self.text(text, *style),
            Kind::Image {
                url,
                alt,
                size,
                fit,
            } => media::picture(&self.shown(url), self.shown(alt), *size, *fit),
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
            } => divider(Orientation::Horizontal),
            Kind::Divider {
                axis: Axis::Vertical,
            } => divider(Orientation::Vertical),
            Kind::Button { child, look, event } => self.button(child, *look, event.as_ref()),
            Kind::Piece(piece) => self.piece(piece),
            Kind::Unsupported { name } => self.mark(format!("Unsupported component \"{name}\".")),
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
        let node = if self.bound(value).is_some() {
            text(self.live(value))
        } else {
            prose::plain_links(&self.shown(value))
        };
        node.wrap().class(size).class(tone.text()).into()
    }

    pub(super) fn children(&self, children: &Children) -> Vec<NodeBuilder> {
        match children {
            Children::Ids(ids) => ids.iter().map(|id| self.node(id)).collect(),
            Children::Template { id, path } => {
                let path = self.path(path);
                let count = untrack(|| self.model.len(&path)).min(MAX_TEMPLATE_ITEMS);
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
            .gap(Space::Sm)
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
            strip = strip.tab(self.shown(title), move || render.node(&child));
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
        if let Some(fire) = event.and_then(|event| self.trigger(event)) {
            control = control.activate(move || fire());
        }
        control.into()
    }

    pub(super) fn trigger(&self, event: &Event) -> Option<Rc<dyn Fn()>> {
        let handler = self.handler.clone()?;
        let name = event.name.clone();
        let context: Vec<(String, Dynamic)> = event.context.clone();
        let render = self.clone();
        Some(Rc::new(move || {
            let mut map = serde_json::Map::new();
            for (key, value) in &context {
                map.insert(key.clone(), render.value(value));
            }
            handler(&name, &Value::Object(map).to_string());
        }))
    }
}
