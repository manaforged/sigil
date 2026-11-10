use std::rc::Rc;

use crate::kernel::NodeBuilder;
use crate::style::Style;

use super::super::layout_ext::LayoutOverrides;
use super::super::shared::Activate;
use super::nav::{PaletteEntry, SubmitObserver};
use crate::widgets::{Edit, Sealed, Widget};

#[derive(Default)]
pub(super) struct LevelDef {
    pub(super) entries: Vec<PaletteEntry>,
}

impl LevelDef {
    fn item(&mut self, label: impl Into<String>, action: impl Fn() + 'static) {
        self.entries.push(PaletteEntry::Leaf {
            label: label.into(),
            detail: None,
            action: Some(Rc::new(action)),
            disabled: false,
        });
    }

    fn detail(&mut self, text: impl Into<String>) {
        if let Some(PaletteEntry::Leaf { detail, .. }) = self.entries.last_mut() {
            *detail = Some(text.into());
        }
    }

    fn submenu(&mut self, label: impl Into<String>, def: Self) {
        self.entries.push(PaletteEntry::Submenu {
            label: label.into(),
            disabled: false,
            entries: Rc::new(def.entries),
        });
    }

    fn disabled(&mut self, label: impl Into<String>) {
        self.entries.push(PaletteEntry::Leaf {
            label: label.into(),
            detail: None,
            action: None,
            disabled: true,
        });
    }
}

/// Builder of a command palette submenu.
#[derive(Default)]
pub struct SubmenuBuilder {
    level: LevelDef,
}

impl core::fmt::Debug for SubmenuBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SubmenuBuilder").finish_non_exhaustive()
    }
}

impl SubmenuBuilder {
    /// Sets the last item's trailing detail, such as a shortcut; search matches it too.
    #[must_use]
    pub fn detail(mut self, text: impl Into<String>) -> Self {
        self.level.detail(text);
        self
    }

    /// Adds an item.
    #[must_use]
    pub fn item(mut self, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        self.level.item(label, action);
        self
    }

    /// Sets the submenu.
    #[must_use]
    pub fn submenu(mut self, label: impl Into<String>, build: impl FnOnce(Self) -> Self) -> Self {
        let nested = build(Self::default()).level;
        self.level.submenu(label, nested);
        self
    }

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, label: impl Into<String>) -> Self {
        self.level.disabled(label);
        self
    }
}

/// A command palette; `layout`, `width`, and `height` size each level panel, and `map_node` changes the root.
#[derive(Default)]
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
#[must_use]
pub fn command_menu() -> CommandMenuBuilder {
    CommandMenuBuilder::default()
}

impl CommandMenuBuilder {
    /// Sets the last item's trailing detail, such as a shortcut; search matches it too.
    #[must_use]
    pub fn detail(mut self, text: impl Into<String>) -> Self {
        self.level.detail(text);
        self
    }

    /// Adds an item.
    #[must_use]
    pub fn item(mut self, label: impl Into<String>, action: impl Fn() + 'static) -> Self {
        self.level.item(label, action);
        self
    }

    /// Sets the submenu.
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

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, label: impl Into<String>) -> Self {
        self.level.disabled(label);
        self
    }

    /// Sets the submit.
    #[must_use]
    pub fn submit(mut self, cb: impl Fn(&str) + 'static) -> Self {
        self.on_submit = Some(Rc::new(cb));
        self
    }

    /// Sets the placeholder.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Sets the cancel.
    #[must_use]
    pub fn cancel(mut self, cb: impl Fn() + 'static) -> Self {
        self.on_cancel = Some(Rc::new(cb));
        self
    }
}

builder_impls!([] CommandMenuBuilder: paint(style_accumulator), node);

impl Sealed for CommandMenuBuilder {
    fn edit(mut self, edit: Edit) -> Self {
        match edit {
            Edit::Layout(edit) => self.panel_layout.edit(edit),
            other => self
                .layout_overrides
                .node_hook(move |node| other.apply(node)),
        }
        self
    }

    fn map_target(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.map_node(f)
    }
}

impl Widget for CommandMenuBuilder {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.layout_overrides.node_hook(f);
        self
    }
}
