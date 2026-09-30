use crate::events::LogicalKey;
use crate::reactive::Reactive;
use std::rc::Rc;

use crate::events::{Key, NodeEventsExt};
use crate::kernel::{AccessibleName, TextContent};
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{GHOST_VARIANT, SOFT_VARIANT, SOLID_VARIANT, Tone};
use crate::style::{Metrics, NodeStyleExt, Radius, Size, Style, StyleAtom, StyleVariant, Theme};

#[cfg(feature = "scene")]
use crate::widgets::icon::icon_svg;
use crate::widgets::interaction::InteractionState;
use crate::widgets::spinner::spinner;

use super::layout_ext::LayoutOverrides;
use super::shared::{
    Activate, BuilderMutator, ICON_GLYPH, activate_key_reactive, activate_pointer_reactive,
    noop_activate, or_disabled, reactive_style_state,
};

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
    square: bool,
    #[cfg(feature = "scene")]
    leading: Option<Rc<str>>,
}

#[must_use]
pub fn button(label: impl Into<TextContent>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

#[must_use]
pub fn icon_button(glyph: impl Into<TextContent>, activate: impl Fn() + 'static) -> ButtonBuilder {
    let mut builder = button(glyph).ghost().activate(activate);
    builder.square = true;
    builder
}

impl ButtonBuilder {
    #[must_use]
    pub fn new(label: impl Into<TextContent>) -> Self {
        let metrics = Metrics::current();
        Self {
            label: label.into(),
            tone: Tone::Neutral,
            variant: SOFT_VARIANT,
            label_atoms: LABEL_TEXT_SOFT,
            frame: true,
            width: ButtonWidth::Hug,
            height: metrics.control_height(Size::Default),
            padding_x: metrics.padding(Size::Default),
            enabled: true,
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
    pub const fn outline(mut self) -> Self {
        self.tone = Tone::Neutral;
        self.variant = GHOST_VARIANT;
        self.label_atoms = LABEL_TEXT_SOFT;
        self.frame = true;
        self
    }

    #[cfg(feature = "scene")]
    #[must_use]
    pub fn icon_svg(mut self, svg: impl Into<Rc<str>>) -> Self {
        self.leading = Some(svg.into());
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
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        let metrics = Metrics::current();
        let size = if size == Size::Grow {
            self.width = ButtonWidth::Fill;
            Size::Default
        } else {
            self.width = ButtonWidth::Hug;
            size
        };
        self.height = metrics.control_height(size);
        self.padding_x = metrics.padding(size);
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
        let interaction = InteractionState::new(self.enabled);
        let activate = self.activate.as_ref().map_or_else(noop_activate, Rc::clone);
        let layout_overrides = self.layout_overrides.clone();
        let user_overlay = self.style_accumulator.take_state_overlay();
        let mut node = self.base_node(
            interaction,
            Rc::clone(&activate),
            user_overlay,
            look_disabled,
        );
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
        let variant = self.variant;
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
            .class(Radius::Lg.atom());
        let node = self
            .framed(node)
            .class(variant.base())
            .class(BUTTON_TRANSITION_ATOMS)
            .tone(self.tone)
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
            None if self.square => node.child(
                label_node(self.label.clone(), self.tone, self.label_atoms, false)
                    .class(ICON_GLYPH),
            ),
            None => self.label_content(node),
        };
        interaction.install(node)
    }

    #[cfg(feature = "scene")]
    fn label_content(&self, node: NodeBuilder) -> NodeBuilder {
        let label = self.label_text();
        let Some(svg) = self.leading.clone() else {
            return node.child(label);
        };
        let size = if self.height < 30.0 { 14.0 } else { 16.0 };
        node.child(
            container()
                .hittable(false)
                .layout(
                    LayoutStyle::flex_row()
                        .align_items(AlignItems::Center)
                        .gap(Metrics::current().gap(Size::Compact)),
                )
                .child(
                    icon_svg(svg)
                        .tone(self.tone)
                        .px(size)
                        .on_solid(self.variant == SOLID_VARIANT),
                )
                .child(label),
        )
    }

    #[cfg(not(feature = "scene"))]
    fn label_content(&self, node: NodeBuilder) -> NodeBuilder {
        node.child(self.label_text())
    }

    fn label_text(&self) -> crate::kernel::TextBuilder {
        let compact = compact_label(self.height, self.layout_overrides.height_hint());
        label_node(self.label.clone(), self.tone, self.label_atoms, compact)
    }

    fn framed(&self, node: NodeBuilder) -> NodeBuilder {
        match (self.frame, self.variant == SOLID_VARIANT) {
            (false, _) => node,
            (true, true) => node.class(SOLID_FRAME),
            (true, false) => node.class(BUTTON_FRAME),
        }
    }

    fn pending_content(&self, pending: Signal<bool>) -> NodeBuilder {
        let (label, tone, atoms) = (self.label.clone(), self.tone, self.label_atoms);
        let compact = compact_label(self.height, self.layout_overrides.height_hint());
        container()
            .hittable(false)
            .layout(
                LayoutStyle::flex_row()
                    .align_items(AlignItems::Center)
                    .gap(Metrics::current().gap(Size::Compact)),
            )
            .class(atoms)
            .children_reactive(move || {
                let mut parts = Vec::with_capacity(2);
                if pending.get() {
                    parts.push(spinner().hittable(false));
                }
                parts.push(container().child(label_node(label.clone(), tone, atoms, compact)));
                parts
            })
    }
}

mod impls;
mod parts;

use parts::{
    BUTTON_FRAME, BUTTON_TRANSITION_ATOMS, ButtonWidth, LABEL_TEXT_SOFT, LABEL_TEXT_SOLID,
    SOLID_FRAME, accessible_label, button_layout, compact_label, label_node,
};

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
