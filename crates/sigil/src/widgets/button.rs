use crate::events::LogicalKey;
use crate::reactive::Reactive;
use std::rc::Rc;

use crate::events::{Key, NodeEventsExt};
use crate::kernel::{AccessibleName, TextContent};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::LayoutBuilderExt;
use crate::reactive::Signal;
use crate::style::{
    ControlSize, Metrics, NodeStyleExt, PaintTarget, Radius, StateHost, Style, StyleSlot, Theme,
    Tone,
};

use crate::widgets::interaction::InteractionState;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{
    Activate, BuilderMutator, activate_key_reactive, activate_pointer_reactive, noop_activate,
    or_disabled, reactive_style_state,
};

pub struct ButtonBuilder {
    label: TextContent,
    tone: Option<Tone>,
    fill: Option<ButtonFill>,
    width: ButtonWidth,
    height: f32,
    padding_x: f32,
    disabled: Option<Signal<bool>>,
    activate: Option<Activate>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
    accessible: Option<AccessibleName>,
    pending: Option<Signal<bool>>,
    square: bool,
    #[cfg(feature = "scene")]
    leading: Option<Rc<str>>,
}

#[must_use]
pub fn button(label: impl Into<TextContent>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

#[must_use]
pub fn icon_button(glyph: impl Into<ButtonGlyph>, activate: impl Fn() + 'static) -> ButtonBuilder {
    let mut builder = glyph.into().into_button().ghost().activate(activate);
    builder.square = true;
    builder
}

impl ButtonBuilder {
    #[must_use]
    pub fn new(label: impl Into<TextContent>) -> Self {
        let metrics = Metrics::current();
        Self {
            label: label.into(),
            tone: None,
            fill: None,
            width: ButtonWidth::Hug,
            height: metrics.control_height(ControlSize::Default),
            padding_x: metrics.padding(ControlSize::Default),
            disabled: None,
            activate: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
            accessible: None,
            pending: None,
            square: false,
            #[cfg(feature = "scene")]
            leading: None,
        }
    }

    #[must_use]
    pub fn accessible(mut self, name: impl Into<AccessibleName>) -> Self {
        self.accessible = Some(name.into());
        self
    }

    #[must_use]
    pub const fn pending(mut self, pending: Signal<bool>) -> Self {
        self.pending = Some(pending);
        self
    }

    #[must_use]
    pub const fn primary(self) -> Self {
        self.tone(Tone::Primary)
    }

    #[must_use]
    pub const fn neutral(self) -> Self {
        self.tone(Tone::Neutral)
    }

    #[must_use]
    pub const fn destructive(self) -> Self {
        self.tone(Tone::Error)
    }

    #[must_use]
    pub const fn outline(self) -> Self {
        self.fill(ButtonFill::Outline)
    }

    #[cfg(feature = "scene")]
    #[must_use]
    pub fn icon_svg(mut self, svg: impl Into<Rc<str>>) -> Self {
        self.leading = Some(svg.into());
        self
    }

    #[cfg(feature = "scene")]
    #[must_use]
    pub fn icon(self, icon: crate::icons::Icon) -> Self {
        self.icon_svg(icon.svg())
    }

    #[must_use]
    pub const fn ghost(self) -> Self {
        self.fill(ButtonFill::Ghost)
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    #[must_use]
    pub const fn solid(self) -> Self {
        self.fill(ButtonFill::Solid)
    }

    #[must_use]
    pub const fn soft(self) -> Self {
        self.fill(ButtonFill::Soft)
    }

    const fn fill(mut self, fill: ButtonFill) -> Self {
        self.fill = Some(fill);
        self
    }

    const fn resolved_tone(&self) -> Tone {
        match self.tone {
            Some(tone) => tone,
            None => Tone::Neutral,
        }
    }

    const fn resolved_fill(&self) -> ButtonFill {
        match (self.fill, self.tone) {
            (Some(fill), _) => fill,
            (None, Some(Tone::Primary | Tone::Error)) => ButtonFill::Solid,
            (None, _) => ButtonFill::Soft,
        }
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn size(mut self, size: ControlSize) -> Self {
        let metrics = Metrics::current();
        let size = if size == ControlSize::Grow {
            self.width = ButtonWidth::Fill;
            ControlSize::Default
        } else {
            self.width = ButtonWidth::Hug;
            size
        };
        self.height = metrics.control_height(size);
        self.padding_x = metrics.padding(size);
        self
    }

    #[must_use]
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let look_disabled = self.disabled;
        self.disabled = or_disabled(self.disabled, self.pending);
        if self.square {
            self.width = ButtonWidth::Fixed(self.height);
            self.padding_x = 0.0;
        }
        let interaction = InteractionState::new(true);
        let activate = self.activate.as_ref().map_or_else(noop_activate, Rc::clone);
        let layout_overrides = self.layout_overrides.clone();
        let user_overlay = self.style_accumulator.take_state_overlay();
        let mut node = self.base_node(interaction, activate, user_overlay, look_disabled);
        for mutate in self.mutators {
            node = mutate(node);
        }
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node.with(layout_overrides.apply(button_layout(self.width, self.height, self.padding_x)))
    }

    fn base_node(
        &self,
        interaction: InteractionState,
        activate: Activate,
        user_overlay: crate::style::StyleOverlay,
        look_disabled: Option<Signal<bool>>,
    ) -> NodeBuilder {
        let variant = self.resolved_fill().variant();
        let disabled = self.disabled;
        let (pointer_activate, key_activate) = (Rc::clone(&activate), Rc::clone(&activate));
        let name = self
            .accessible
            .clone()
            .unwrap_or_else(|| accessible_label(&self.label));
        let node = container()
            .role(Role::Button)
            .mutate(move |node| node.a11y.disabled = disabled)
            .label(name)
            .layout(button_layout(self.width, self.height, self.padding_x))
            .class(Radius::Lg.atom());
        let node = self
            .framed(node)
            .class(variant.base())
            .class(BUTTON_TRANSITION_ATOMS)
            .tone(self.resolved_tone())
            .stateful_style(variant, user_overlay, move || {
                reactive_style_state(interaction, look_disabled)
            })
            .seal_style()
            .pointer_up(move |event| {
                activate_pointer_reactive(event, interaction, disabled, &pointer_activate);
            })
            .key(Key::Enter, move |event| {
                activate_key_reactive(event, interaction, disabled, &key_activate);
            })
            .key(LogicalKey::character(" "), move |event| {
                activate_key_reactive(event, interaction, disabled, &activate);
            });
        let node = match self.pending {
            Some(pending) => node.child(self.pending_content(pending)),
            None if self.square => node.child(self.square_content()),
            None => self.label_content(node),
        };
        interaction.install(node)
    }
}

impl PaintTarget for ButtonBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for ButtonBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl StateHost for ButtonBuilder {}

impl IntoNode for ButtonBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

mod content;
mod glyph;
mod parts;

pub use glyph::ButtonGlyph;

use parts::{BUTTON_TRANSITION_ATOMS, ButtonFill, ButtonWidth, accessible_label, button_layout};

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
