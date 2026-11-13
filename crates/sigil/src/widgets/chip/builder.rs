use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role};
use crate::layout::snap::line;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::{Reactive, Signal};
use crate::style::{Radius, Style, Tone, space};
use crate::widgets::Widget;

use super::super::dot::dot;
use super::super::shared::{Activate, accessible_name, caller_style};
use super::chip_class;

/// Builder of a chip.
#[must_use]
pub struct ChipBuilder {
    label: crate::kernel::TextContent,
    tone: Tone,
    tone_signal: Option<Signal<Tone>>,
    activate: Option<Activate>,
    leading: Option<Box<dyn IntoNode>>,
    dot: bool,
    style_accumulator: Style,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for ChipBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ChipBuilder")
            .field("label", &self.label)
            .field("tone", &self.tone)
            .field("tone_signal", &self.tone_signal)
            .field("dot", &self.dot)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

impl ChipBuilder {
    pub(crate) fn new(label: impl Into<crate::kernel::TextContent>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
            tone_signal: None,
            activate: None,
            leading: None,
            dot: false,
            style_accumulator: Style::new(),
            layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
        }
    }

    /// Sets the color tone; a signal recolors the chip when its value changes.
    pub fn tone(mut self, tone: impl Reactive<Tone>) -> Self {
        self.tone = tone.read();
        self.tone_signal = Some(tone.into_signal());
        self
    }

    /// Shows `node` before the label, after the dot if there is one.
    pub fn leading(mut self, node: impl IntoNode + 'static) -> Self {
        self.leading = Some(Box::new(node));
        self
    }

    /// Shows a status dot in the chip's tone before the label.
    pub const fn dot(mut self) -> Self {
        self.dot = true;
        self
    }

    /// Runs `activate` when the user activates the chip, and gives the chip the button role.
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

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
                    .padding_x(Space::Sm)
                    .gap(Space::Xs)
                    .align_items(AlignItems::Center)
                    .shrink(0.0),
            )
            .class("text-xs font-medium")
            .class(Radius::Full.atom())
            .class(chip_class(self.tone));
        if activate.is_some() {
            node = node.label(accessible_name(&self.label));
        }
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
            node = node.activate_with(activate);
        }
        let node = match self.tone_signal {
            Some(tone) => node.style_reactive(move || chip_class(tone.get())),
            None => node,
        };
        caller_style(node, self.style_accumulator)
    }
}

builder_impls!([] ChipBuilder: paint(style_accumulator), states(style_accumulator), widget, node(pub));
