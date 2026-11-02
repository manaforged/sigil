use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role};
use crate::layout::snap::line;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::{Reactive, Signal};
use crate::style::{Radius, Tone, space};
use crate::widgets::Widget;

use crate::widgets::interaction::InteractionState;

use super::super::dot::dot;
use super::super::shared::{Activate, Press, RELEASE_ENTER, press};
use super::chip_class;

/// Builder of a chip.
pub struct ChipBuilder {
    label: crate::kernel::TextContent,
    tone: Tone,
    tone_signal: Option<Signal<Tone>>,
    activate: Option<Activate>,
    leading: Option<Box<dyn IntoNode>>,
    dot: bool,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl ChipBuilder {
    #[must_use]
    pub(crate) fn new(label: impl Into<crate::kernel::TextContent>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
            tone_signal: None,
            activate: None,
            leading: None,
            dot: false,
            layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
        }
    }

    /// Sets the tone.
    #[must_use]
    pub fn tone(mut self, tone: impl Reactive<Tone>) -> Self {
        self.tone = tone.read();
        self.tone_signal = Some(tone.into_signal());
        self
    }

    /// Sets the leading.
    #[must_use]
    pub fn leading(mut self, node: impl IntoNode + 'static) -> Self {
        self.leading = Some(Box::new(node));
        self
    }

    /// Applies the dot option.
    #[must_use]
    pub const fn dot(mut self) -> Self {
        self.dot = true;
        self
    }

    /// Sets the activate.
    #[must_use]
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    #[must_use]
    fn root(self) -> NodeBuilder {
        let activate = self.activate;
        let mut node = crate::kernel::container()
            .role(if activate.is_some() {
                Role::Button
            } else {
                Role::Label
            })
            .layout(
                LayoutStyle::flex_row()
                    .height(line(space(5)))
                    .padding_x(space(2))
                    .gap(space(1))
                    .align_items(AlignItems::Center)
                    .shrink(0.0),
            )
            .class("text-xs font-medium")
            .class(Radius::Full.atom())
            .class(chip_class(self.tone));
        if self.dot {
            node = match self.tone_signal {
                Some(tone) => node
                    .child(crate::kernel::container().children_reactive(move || [dot(tone.get())])),
                None => node.child(dot(self.tone)),
            };
        }
        if let Some(leading) = self.leading {
            node = node.child(leading);
        }
        node = node.child(crate::kernel::text(self.label).nowrap());
        if let Some(activate) = activate {
            let interaction = InteractionState::new(true);
            node = press(
                node,
                interaction,
                None,
                Press::Activate(activate),
                RELEASE_ENTER,
            );
            node = interaction.install(node);
        }
        match self.tone_signal {
            Some(tone) => node.style_reactive(move || chip_class(tone.get())),
            None => node,
        }
    }
}

builder_impls!([] ChipBuilder: widget, node(pub));
