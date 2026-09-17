use crate::hal::LogicalKey;
use std::rc::Rc;

use crate::kernel::{AccessibleName, TextContent};
use crate::reactive::Signal;
use crate::style::{GHOST_VARIANT, SOFT_VARIANT, SOLID_VARIANT, Tone};
use crate::widgets::widget_dev::*;

use crate::widgets::interaction::InteractionState;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{
    Activate, BuilderMutator, activate_key_reactive, activate_pointer_reactive, noop_activate,
    or_disabled, reactive_style_state,
};
use super::transition_seam::install_transition_animators;

const BUTTON_COMPONENT_ID: u64 = 0xB117_7000;

pub struct ButtonBuilder {
    label: TextContent,
    tone: Tone,
    variant: StyleVariant,
    label_atoms: &'static [StyleAtom],
    frame: bool,
    width: ButtonWidth,
    height: f32,
    padding_x: f32,
    enabled: bool,
    disabled: Option<Signal<bool>>,
    activate: Option<Activate>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
    accessible: Option<AccessibleName>,
    pending: Option<Signal<bool>>,
}

#[must_use]
pub fn button(label: impl Into<TextContent>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

#[must_use]
pub fn icon_button(glyph: impl Into<TextContent>, activate: impl Fn() + 'static) -> ButtonBuilder {
    button(glyph).ghost().activate(activate)
}

impl ButtonBuilder {
    #[must_use]
    pub fn new(label: impl Into<TextContent>) -> Self {
        let theme = Theme::default();
        Self {
            label: label.into(),
            tone: Tone::Neutral,
            variant: SOFT_VARIANT,
            label_atoms: LABEL_TEXT_SOFT,
            frame: true,
            width: ButtonWidth::Hug,
            height: theme.control_height(Size::Default).px_or_zero(),
            padding_x: theme.padding(Size::Default).px_or_zero(),
            enabled: true,
            disabled: None,
            activate: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
            accessible: None,
            pending: None,
        }
    }

    #[must_use]
    pub fn accessible(mut self, name: impl Into<String>) -> Self {
        self.accessible = Some(AccessibleName::Static(name.into()));
        self
    }

    #[must_use]
    pub const fn pending(mut self, pending: Signal<bool>) -> Self {
        self.pending = Some(pending);
        self
    }

    #[must_use]
    pub const fn primary(mut self) -> Self {
        self.tone = Tone::Primary;
        self.variant = SOLID_VARIANT;
        self.label_atoms = LABEL_TEXT_SOLID;
        self
    }

    #[must_use]
    pub const fn neutral(mut self) -> Self {
        self.tone = Tone::Neutral;
        self.variant = SOFT_VARIANT;
        self.label_atoms = LABEL_TEXT_SOFT;
        self.frame = true;
        self
    }

    #[must_use]
    pub const fn destructive(mut self) -> Self {
        self.tone = Tone::Error;
        self.variant = SOLID_VARIANT;
        self.label_atoms = LABEL_TEXT_SOLID;
        self.frame = true;
        self
    }

    #[must_use]
    pub const fn ghost(mut self) -> Self {
        self.tone = Tone::Neutral;
        self.variant = GHOST_VARIANT;
        self.label_atoms = LABEL_TEXT_SOFT;
        self.frame = false;
        self
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub const fn solid(mut self) -> Self {
        self.variant = SOLID_VARIANT;
        self.label_atoms = LABEL_TEXT_SOLID;
        self.frame = true;
        self
    }

    #[must_use]
    pub const fn soft(mut self) -> Self {
        self.variant = SOFT_VARIANT;
        self.label_atoms = LABEL_TEXT_SOFT;
        self.frame = true;
        self
    }

    #[must_use]
    pub const fn enabled(mut self, yes: bool) -> Self {
        self.enabled = yes;
        self
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        let theme = Theme::default();
        if size == Size::Grow {
            self.width = ButtonWidth::Fill;
            self.height = theme.control_height(Size::Default).px_or_zero();
            self.padding_x = theme.padding(Size::Default).px_or_zero();
        } else {
            self.width = ButtonWidth::Hug;
            self.height = theme.control_height(size).px_or_zero();
            self.padding_x = theme.padding(size).px_or_zero();
        }
        self
    }

    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = ButtonWidth::Fixed(width);
        self
    }

    #[must_use]
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        self.disabled = or_disabled(self.disabled, self.pending);
        let interaction = InteractionState::new(self.enabled);
        let activate = self.activate.as_ref().map_or_else(noop_activate, Rc::clone);
        let layout_overrides = self.layout_overrides.clone();
        let user_overlay = self.style_accumulator.take_state_overlay();
        let mut node = self.base_node(interaction, Rc::clone(&activate), user_overlay);
        for mutate in self.mutators {
            node = mutate(node);
        }
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node.layout(layout_overrides.apply(button_layout(self.width, self.height, self.padding_x)))
    }

    fn base_node(
        &self,
        interaction: InteractionState,
        activate: Activate,
        user_overlay: crate::style::StyleOverlay,
    ) -> NodeBuilder {
        let variant = self.variant;
        let state_style = variant.states();
        let disabled = self.disabled;
        let pointer_activate = Rc::clone(&activate);
        let key_activate = Rc::clone(&activate);
        let name = self
            .accessible
            .clone()
            .unwrap_or_else(|| accessible_label(&self.label));
        let node = container()
            .role(Role::Button)
            .mutate(move |node| node.a11y.disabled = disabled)
            .label(name)
            .layout(button_layout(self.width, self.height, self.padding_x))
            .class("rounded-lg");
        let frame_atoms = if self.variant == SOLID_VARIANT {
            SOLID_FRAME
        } else {
            BUTTON_FRAME
        };
        let node = if self.frame {
            node.class(frame_atoms)
        } else {
            node
        };
        let node = node
            .class(variant.base())
            .class(BUTTON_TRANSITION_ATOMS)
            .tone(self.tone)
            .stateful_style(variant, user_overlay, move || {
                reactive_style_state(interaction, disabled)
            })
            .pointer_up(move |event| {
                activate_pointer_reactive(event, interaction, disabled, &pointer_activate);
            })
            .key(Key::Enter, move |event| {
                activate_key_reactive(event, interaction, disabled, &key_activate);
            })
            .key(LogicalKey::character(" "), move |event| {
                activate_key_reactive(event, interaction, disabled, &activate);
            })
            .child(self.label_node());
        let node = install_transition_animators(
            node,
            variant.base(),
            state_style,
            BUTTON_COMPONENT_ID,
            interaction,
        );
        interaction.install(node)
    }

    fn label_node(&self) -> TextBuilder {
        text(self.label.clone())
            .nowrap()
            .tone(self.tone)
            .class(self.label_atoms)
    }
}

impl StyleAccess for ButtonBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
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
        self.into_node_builder().build(cx)
    }
}

mod parts;

use parts::{
    BUTTON_FRAME, ButtonWidth, LABEL_TEXT_SOFT, LABEL_TEXT_SOLID, accessible_label, button_layout,
};
use parts::{BUTTON_TRANSITION_ATOMS, SOLID_FRAME};

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
