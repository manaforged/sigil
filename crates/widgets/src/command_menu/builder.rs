use std::rc::Rc;

use sigil_kernel::{Cx, IntoNode};
use sigil_style::{Style, StyleAccess};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::Activate;
use super::nav::SelectObserver;
use super::render::into_node_builder;

pub(super) enum EntryDef {
    Leaf {
        label: String,
        action: Option<Activate>,
        disabled: bool,
    },
    Submenu {
        label: String,
        disabled: bool,
        def: LevelDef,
    },
}

#[derive(Default)]
pub(super) struct LevelDef {
    pub(super) entries: Vec<EntryDef>,
}

impl LevelDef {
    fn item(&mut self, label: impl Into<String>, action: impl Fn() + 'static) {
        self.entries.push(EntryDef::Leaf {
            label: label.into(),
            action: Some(Rc::new(action)),
            disabled: false,
        });
    }

    fn submenu(&mut self, label: impl Into<String>, def: Self) {
        self.entries.push(EntryDef::Submenu {
            label: label.into(),
            disabled: false,
            def,
        });
    }

    fn disabled(&mut self, label: impl Into<String>) {
        self.entries.push(EntryDef::Leaf {
            label: label.into(),
            action: None,
            disabled: true,
        });
    }
}

#[derive(Default)]
pub struct SubmenuBuilder {
    level: LevelDef,
}

impl SubmenuBuilder {
    #[must_use]
    pub fn item(mut self, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        self.level.item(label, action);
        self
    }

    #[must_use]
    pub fn submenu(mut self, label: impl Into<String>, build: impl FnOnce(Self) -> Self) -> Self {
        let nested = build(Self::default()).level;
        self.level.submenu(label, nested);
        self
    }

    #[must_use]
    pub fn disabled(mut self, label: impl Into<String>) -> Self {
        self.level.disabled(label);
        self
    }
}

#[derive(Default)]
pub struct CommandMenuBuilder {
    level: LevelDef,
    on_select: Option<SelectObserver>,
    on_cancel: Option<Activate>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn command_menu() -> CommandMenuBuilder {
    CommandMenuBuilder::default()
}

impl CommandMenuBuilder {
    #[must_use]
    pub fn item(mut self, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        self.level.item(label, action);
        self
    }

    #[must_use]
    pub fn submenu(
        mut self,
        label: impl Into<String>,
        build: impl FnOnce(SubmenuBuilder) -> SubmenuBuilder,
    ) -> Self {
        let nested = build(SubmenuBuilder::default()).level;
        self.level.submenu(label, nested);
        self
    }

    #[must_use]
    pub fn disabled(mut self, label: impl Into<String>) -> Self {
        self.level.disabled(label);
        self
    }

    #[must_use]
    pub fn select(mut self, cb: impl Fn(&str) + 'static) -> Self {
        self.on_select = Some(Rc::new(cb));
        self
    }

    #[must_use]
    pub fn cancel(mut self, cb: impl Fn() + 'static) -> Self {
        self.on_cancel = Some(Rc::new(cb));
        self
    }
}

impl WidgetLayoutExt for CommandMenuBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl StyleAccess for CommandMenuBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for CommandMenuBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        into_node_builder(
            self.level,
            self.on_select,
            self.on_cancel,
            self.style_accumulator,
            self.layout_overrides,
        )
        .build(cx)
    }
}
