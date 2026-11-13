use std::rc::Rc;

use crate::kernel::NodeBuilder;
use crate::style::Style;

use super::super::layout_ext::LayoutOverrides;
use super::super::shared::Activate;
use super::nav::{PaletteEntry, SubmitObserver};
use crate::widgets::{MenuItem, Widget};

#[derive(Default)]
pub(super) struct LevelDef {
    pub(super) entries: Vec<PaletteEntry>,
}

impl LevelDef {
    fn item(&mut self, item: MenuItem) {
        self.entries.push(PaletteEntry::Leaf {
            label: item.label,
            detail: item.hint,
            action: item.on_activate,
            disabled: item.disabled,
        });
    }

    fn submenu(&mut self, label: impl Into<String>, def: Self) {
        self.entries.push(PaletteEntry::Submenu {
            label: label.into(),
            disabled: false,
            entries: Rc::new(def.entries),
        });
    }
}

/// Builder of a command palette submenu.
#[derive(Default)]
#[must_use]
pub struct SubmenuBuilder {
    level: LevelDef,
}

impl core::fmt::Debug for SubmenuBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SubmenuBuilder").finish_non_exhaustive()
    }
}

macro_rules! level_builder_methods {
    () => {
        /// Adds an item; its hint shows after the label and search matches it, and its submenu mark is not drawn.
        pub fn item(mut self, item: MenuItem) -> Self {
            self.level.item(item);
            self
        }

        /// Adds an item that opens a nested level, which `build` fills.
        pub fn submenu(
            mut self,
            label: impl Into<String>,
            build: impl FnOnce(SubmenuBuilder) -> SubmenuBuilder,
        ) -> Self {
            let nested = build(SubmenuBuilder::default()).level;
            self.level.submenu(label, nested);
            self
        }
    };
}

impl SubmenuBuilder {
    level_builder_methods!();
}

/// A command palette; `layout`, `width`, and `height` size each level panel, and `map_node` changes the root.
#[derive(Default)]
#[must_use]
pub struct CommandMenuBuilder {
    pub(super) level: LevelDef,
    pub(super) on_submit: Option<SubmitObserver>,
    pub(super) on_cancel: Option<Activate>,
    pub(super) placeholder: String,
    pub(super) style_accumulator: Style,
    pub(super) panel_layout: LayoutOverrides,
    layout_overrides: LayoutOverrides,
}

impl core::fmt::Debug for CommandMenuBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CommandMenuBuilder")
            .field("placeholder", &self.placeholder)
            .field("style_accumulator", &self.style_accumulator)
            .field("panel_layout", &self.panel_layout)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A command palette.
pub fn command_menu() -> CommandMenuBuilder {
    CommandMenuBuilder::default()
}

impl CommandMenuBuilder {
    level_builder_methods!();

    /// Runs `cb` with the label of an item after the user chooses it and its action runs.
    pub fn submit(mut self, cb: impl Fn(&str) + 'static) -> Self {
        self.on_submit = Some(Rc::new(cb));
        self
    }

    /// Sets the placeholder text of the search field.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Runs `cb` when the user closes the palette while the top level is shown.
    pub fn cancel(mut self, cb: impl Fn() + 'static) -> Self {
        self.on_cancel = Some(Rc::new(cb));
        self
    }
}

builder_impls!([] CommandMenuBuilder: paint(style_accumulator), node);

impl Widget for CommandMenuBuilder {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.layout_overrides.node_hook(f);
        self
    }

    crate::widgets::widget_layout_methods!(@edit panel_layout.edit);
}

crate::widgets::impl_layout!([] CommandMenuBuilder, @edit panel_layout.edit);
