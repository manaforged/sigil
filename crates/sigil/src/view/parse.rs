use core::fmt;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::kernel::TextContent;
use crate::layout::Space;
use crate::reactive::Signal;
use crate::style::parser::parse_class_string;
use crate::style::{Layer, StyleAtom, TextRole};

use super::Binds;
use super::lex::{Tok, Token, lex};
use super::vocab::{self, Body, Head, Kind, Prop, Spec};

const MAX_DEPTH: usize = 64;

/// Line, column, and reason of a view file problem.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ViewError {
    line: u32,
    col: u32,
    message: String,
}

impl ViewError {
    pub(super) fn at(line: u32, col: u32, message: impl Into<String>) -> Self {
        Self {
            line,
            col,
            message: message.into(),
        }
    }
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {} column {}: {}",
            self.line, self.col, self.message
        )
    }
}

#[derive(Clone)]
pub(super) enum Content {
    Literal(String),
    Bound(Signal<String>),
}

impl From<&Content> for TextContent {
    fn from(content: &Content) -> Self {
        match content {
            Content::Literal(text) => text.clone().into(),
            Content::Bound(signal) => (*signal).into(),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Variant {
    Primary,
    Outline,
    Ghost,
}

pub(super) type Action = Rc<dyn Fn()>;
pub(super) type Slot = Rc<dyn Fn() -> crate::kernel::NodeBuilder>;

#[derive(Default)]
pub(super) struct Attrs {
    pub(super) gap: Option<Space>,
    pub(super) padding: Option<Space>,
    pub(super) layer: Option<Layer>,
    pub(super) role: Option<TextRole>,
    pub(super) class: Option<SmallVec<[StyleAtom; 16]>>,
    pub(super) show: Option<Signal<bool>>,
    pub(super) variant: Option<Variant>,
    pub(super) activate: Option<Action>,
    pub(super) value: Option<Signal<String>>,
    pub(super) checked: Option<Signal<bool>>,
    pub(super) placeholder: Option<Content>,
    pub(super) disabled: Option<Signal<bool>>,
}

pub(super) struct Item {
    pub(super) kind: Kind,
    pub(super) label: Option<Content>,
    pub(super) slot: Option<Slot>,
    pub(super) attrs: Attrs,
    pub(super) children: Vec<Rc<Item>>,
}

struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    at: usize,
    binds: &'a Binds,
}

pub(super) fn parse(source: &str, binds: &Binds) -> Result<Vec<Rc<Item>>, ViewError> {
    let mut parser = Parser {
        tokens: lex(source)?,
        at: 0,
        binds,
    };
    let items = parser.items(0)?;
    if parser.peek().tok != Tok::End {
        return Err(parser.fail("this `}` has no matching `{`"));
    }
    Ok(items)
}

fn step<T: Copy>(table: &[(&'static str, T)], word: &str) -> Option<T> {
    table
        .iter()
        .find(|(name, _)| *name == word)
        .map(|(_, value)| *value)
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Token<'a> {
        &self.tokens[self.at.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Tok<'a> {
        let tok = self.peek().tok.clone();
        self.at += 1;
        tok
    }

    fn fail(&self, message: impl Into<String>) -> ViewError {
        let at = self.peek();
        ViewError::at(at.line, at.col, message)
    }

    fn items(&mut self, depth: usize) -> Result<Vec<Rc<Item>>, ViewError> {
        if depth > MAX_DEPTH {
            return Err(self.fail(format!("widgets nest deeper than {MAX_DEPTH} levels")));
        }
        let mut items = Vec::new();
        loop {
            while self.peek().tok == Tok::Newline {
                self.at += 1;
            }
            if matches!(self.peek().tok, Tok::End | Tok::Close) {
                return Ok(items);
            }
            items.push(Rc::new(self.item(depth)?));
        }
    }

    fn item(&mut self, depth: usize) -> Result<Item, ViewError> {
        let start = self.peek().clone();
        let Tok::Word(word) = start.tok else {
            return Err(self.fail("expected a widget name"));
        };
        let Some(spec) = vocab::WIDGETS.iter().find(|spec| spec.name == word) else {
            let names = vocab::widget_names();
            return Err(self.fail(format!("`{word}` is not a widget: use one of {names}")));
        };
        self.at += 1;
        let (label, slot) = self.head(spec)?;
        let attrs = self.attrs(spec)?;
        if spec.required == Some(Prop::Value) && attrs.value.is_none() {
            return Err(ViewError::at(
                start.line,
                start.col,
                "text_field needs value=$name",
            ));
        }
        if spec.required == Some(Prop::Checked) && attrs.checked.is_none() {
            return Err(ViewError::at(
                start.line,
                start.col,
                "checkbox needs checked=$name",
            ));
        }
        let children = self.children(spec, depth)?;
        if spec.body == Body::One && children.len() != 1 {
            let reason = format!(
                "{word} holds one control and this one has {}",
                children.len()
            );
            return Err(ViewError::at(start.line, start.col, reason));
        }
        Ok(Item {
            kind: spec.kind,
            label,
            slot,
            attrs,
            children,
        })
    }

    fn head(&mut self, spec: &Spec) -> Result<(Option<Content>, Option<Slot>), ViewError> {
        match spec.head {
            Head::Nothing => Ok((None, None)),
            Head::Name => Ok((None, Some(self.slot()?))),
            Head::Label => {
                if !matches!(self.peek().tok, Tok::Str(_) | Tok::Bound(_)) {
                    let name = spec.name;
                    return Err(self.fail(format!("{name} needs its text, as \"words\" or $name")));
                }
                Ok((Some(self.content()?), None))
            }
            Head::OptionalLabel => {
                let next = &self.peek().tok;
                let has_label = matches!(next, Tok::Str(_) | Tok::Bound(_));
                Ok((has_label.then(|| self.content()).transpose()?, None))
            }
        }
    }

    fn slot(&mut self) -> Result<Slot, ViewError> {
        let Tok::Word(name) = self.peek().tok else {
            return Err(self.fail("slot needs a name"));
        };
        let Some(slot) = self.binds.slots.get(name) else {
            return Err(self.fail(format!(
                "no slot named `{name}`: add .slot(\"{name}\", ...)"
            )));
        };
        let slot = Rc::clone(slot);
        self.at += 1;
        Ok(slot)
    }

    fn content(&mut self) -> Result<Content, ViewError> {
        match self.peek().tok.clone() {
            Tok::Str(text) => {
                self.at += 1;
                Ok(Content::Literal(text))
            }
            Tok::Bound(_) => Ok(Content::Bound(self.text_signal()?)),
            _ => Err(self.fail("expected \"words\" or $name")),
        }
    }

    fn text_signal(&mut self) -> Result<Signal<String>, ViewError> {
        let Tok::Bound(name) = self.peek().tok else {
            return Err(self.fail("expected $name"));
        };
        let Some(signal) = self.binds.signals.get(name) else {
            return Err(self.fail(format!(
                "no signal named `{name}`: add .signal(\"{name}\", ...)"
            )));
        };
        let signal = *signal;
        self.at += 1;
        Ok(signal)
    }

    fn flag(&mut self) -> Result<Signal<bool>, ViewError> {
        let Tok::Bound(name) = self.peek().tok else {
            return Err(self.fail("expected $name"));
        };
        let Some(flag) = self.binds.flags.get(name) else {
            return Err(self.fail(format!(
                "no flag named `{name}`: add .flag(\"{name}\", ...)"
            )));
        };
        let flag = *flag;
        self.at += 1;
        Ok(flag)
    }

    fn action(&mut self) -> Result<Action, ViewError> {
        let Tok::Action(name) = self.peek().tok else {
            return Err(self.fail("expected @name"));
        };
        let Some(action) = self.binds.actions.get(name) else {
            return Err(self.fail(format!(
                "no action named `{name}`: add .action(\"{name}\", ...)"
            )));
        };
        let action = Rc::clone(action);
        self.at += 1;
        Ok(action)
    }

    fn table<T: Copy>(&mut self, prop: &str, table: &[(&'static str, T)]) -> Result<T, ViewError> {
        let found = match self.peek().tok {
            Tok::Word(word) => step(table, word),
            _ => None,
        };
        let Some(found) = found else {
            return Err(self.fail(format!("{prop} takes one of {}", vocab::names(table))));
        };
        self.at += 1;
        Ok(found)
    }

    fn class(&mut self) -> Result<SmallVec<[StyleAtom; 16]>, ViewError> {
        let Tok::Str(classes) = self.peek().tok.clone() else {
            return Err(self.fail("class takes \"words\""));
        };
        let atoms = parse_class_string(&classes).map_err(|err| self.fail(err.to_string()))?;
        self.at += 1;
        Ok(atoms)
    }

    fn attrs(&mut self, spec: &Spec) -> Result<Attrs, ViewError> {
        let mut attrs = Attrs::default();
        while let Tok::Word(word) = self.peek().tok {
            let Some(prop) = step(vocab::PROPS, word).filter(|prop| spec.allows(*prop)) else {
                return Err(self.fail(format!(
                    "{} has no property `{word}`; a new widget starts on its own line",
                    spec.name
                )));
            };
            self.at += 1;
            self.prop(prop, word, &mut attrs)?;
        }
        if matches!(
            self.peek().tok,
            Tok::Newline | Tok::Open | Tok::Close | Tok::End
        ) {
            return Ok(attrs);
        }
        Err(self.fail("expected a property or the end of the line"))
    }

    fn prop(&mut self, prop: Prop, word: &str, attrs: &mut Attrs) -> Result<(), ViewError> {
        let variant = match prop {
            Prop::Primary => Some(Variant::Primary),
            Prop::Outline => Some(Variant::Outline),
            Prop::Ghost => Some(Variant::Ghost),
            _ => None,
        };
        if variant.is_some() {
            attrs.variant = variant;
            return Ok(());
        }
        if self.advance() != Tok::Equals {
            self.at -= 1;
            return Err(self.fail(format!("{word} needs `=` and a value")));
        }
        self.value(prop, word, attrs)
    }

    fn value(&mut self, prop: Prop, word: &str, attrs: &mut Attrs) -> Result<(), ViewError> {
        match prop {
            Prop::Gap => attrs.gap = Some(self.table(word, vocab::SPACES)?),
            Prop::Padding => attrs.padding = Some(self.table(word, vocab::SPACES)?),
            Prop::Layer => attrs.layer = Some(self.table(word, vocab::LAYERS)?),
            Prop::Role => attrs.role = Some(self.table(word, vocab::ROLES)?),
            Prop::Class => attrs.class = Some(self.class()?),
            Prop::Show => attrs.show = Some(self.flag()?),
            Prop::Disabled => attrs.disabled = Some(self.flag()?),
            Prop::Checked => attrs.checked = Some(self.flag()?),
            Prop::Value => attrs.value = Some(self.text_signal()?),
            Prop::Placeholder => attrs.placeholder = Some(self.content()?),
            Prop::Activate => attrs.activate = Some(self.action()?),
            Prop::Primary | Prop::Outline | Prop::Ghost => {}
        }
        Ok(())
    }

    fn children(&mut self, spec: &Spec, depth: usize) -> Result<Vec<Rc<Item>>, ViewError> {
        if self.peek().tok != Tok::Open {
            return Ok(Vec::new());
        }
        if spec.body == Body::Leaf {
            return Err(self.fail(format!("{} holds no children", spec.name)));
        }
        self.at += 1;
        let children = self.items(depth + 1)?;
        if self.peek().tok != Tok::Close {
            return Err(self.fail("this `{` has no matching `}`"));
        }
        self.at += 1;
        Ok(children)
    }
}
