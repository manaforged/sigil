use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{AccessibleName, TextContent};
use crate::kernel::{NodeBuilder, Role, container};
use crate::reactive::Signal;
use crate::style::{ControlSize, Metrics, Radius, Style, Tone};
use crate::widgets::Widget;

use crate::widgets::interaction::InteractionState;

use super::layout_ext::LayoutOverrides;
use super::shared::{
    Activate, Press, RELEASE_ENTER_SPACE, accessible_name, control_root, noop_activate,
    or_disabled, press, reactive_style_state,
};

/// Builder of a button.
pub struct ButtonBuilder {
    label: TextContent,
    tone: Option<Tone>,
    fill: Option<ButtonFill>,
    width: ButtonWidth,
    height: f32,
    padding_x: f32,
    disabled: Option<Signal<bool>>,
    activate: Option<Activate>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
    accessible: Option<AccessibleName>,
    pending: Option<Signal<bool>>,
    square: bool,
    #[cfg(feature = "scene")]
    leading: Option<std::sync::Arc<str>>,
    #[cfg(feature = "scene")]
    trailing: Option<std::sync::Arc<str>>,
    shortcut: Option<String>,
}

impl core::fmt::Debug for ButtonBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ButtonBuilder")
            .field("label", &self.label)
            .field("tone", &self.tone)
            .field("fill", &self.fill)
            .field("height", &self.height)
            .field("padding_x", &self.padding_x)
            .field("disabled", &self.disabled)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .field("accessible", &self.accessible)
            .field("pending", &self.pending)
            .field("square", &self.square)
            .field("shortcut", &self.shortcut)
            .finish_non_exhaustive()
    }
}

/// A push button.
#[must_use]
pub fn button(label: impl Into<TextContent>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

/// A button that shows one glyph: text or an `Icon`.
#[must_use]
pub fn icon_button(glyph: impl Into<ButtonGlyph>, activate: impl Fn() + 'static) -> ButtonBuilder {
    let mut builder = glyph.into().into_button().ghost().activate(activate);
    builder.square = true;
    builder
}

impl ButtonBuilder {
    /// Creates a button builder.
    #[must_use]
    pub fn new(label: impl Into<TextContent>) -> Self {
        let metrics = Metrics::current();
        Self {
            label: label.into(),
            tone: None,
            fill: None,
            width: ButtonWidth::Hug,
            height: metrics.control_height(ControlSize::Default),
            padding_x: metrics.control_padding(ControlSize::Default),
            disabled: None,
            activate: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
            accessible: None,
            pending: None,
            square: false,
            #[cfg(feature = "scene")]
            leading: None,
            #[cfg(feature = "scene")]
            trailing: None,
            shortcut: None,
        }
    }

    /// Sets the accessible.
    #[must_use]
    pub fn accessible(mut self, name: impl Into<AccessibleName>) -> Self {
        self.accessible = Some(name.into());
        self
    }

    /// Sets the pending.
    #[must_use]
    pub const fn pending(mut self, pending: Signal<bool>) -> Self {
        self.pending = Some(pending);
        self
    }

    /// Applies the primary option.
    #[must_use]
    pub const fn primary(self) -> Self {
        self.tone(Tone::Primary)
    }

    /// Applies the neutral option.
    #[must_use]
    pub const fn neutral(self) -> Self {
        self.tone(Tone::Neutral)
    }

    /// Applies the destructive option.
    #[must_use]
    pub const fn destructive(self) -> Self {
        self.tone(Tone::Error)
    }

    /// Applies the outline option.
    #[must_use]
    pub const fn outline(self) -> Self {
        self.fill(ButtonFill::Outline)
    }

    /// Sets the icon svg.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn icon_svg(mut self, svg: impl Into<std::sync::Arc<str>>) -> Self {
        self.leading = Some(svg.into());
        self
    }

    /// Sets the icon.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn icon(self, icon: crate::icons::Icon) -> Self {
        self.icon_svg(icon.svg())
    }

    /// Sets the icon shown after the label, such as a dropdown chevron.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn trailing(mut self, icon: crate::icons::Icon) -> Self {
        self.trailing = Some(icon.svg().into());
        self
    }

    /// Sets the key that triggers this button, shown faint after the label.
    #[must_use]
    pub fn shortcut(mut self, key: impl Into<String>) -> Self {
        self.shortcut = Some(key.into());
        self
    }

    /// Applies the ghost option.
    #[must_use]
    pub const fn ghost(self) -> Self {
        self.fill(ButtonFill::Ghost)
    }

    /// Sets the tone.
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Applies the solid option.
    #[must_use]
    pub const fn solid(self) -> Self {
        self.fill(ButtonFill::Solid)
    }

    /// Applies the soft option.
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

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the size.
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
        self.padding_x = metrics.control_padding(size);
        self
    }

    /// Sets the activate.
    #[must_use]
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    fn root(mut self) -> NodeBuilder {
        let look_disabled = self.disabled;
        self.disabled = or_disabled(self.disabled, self.pending);
        if self.square {
            self.width = ButtonWidth::Fixed(self.height);
            self.padding_x = 0.0;
        }
        let activate = self.activate.as_ref().map_or_else(noop_activate, Rc::clone);
        let style = std::mem::take(&mut self.style_accumulator);
        control_root(self.disabled, style, |interaction, overlay| {
            self.base_node(interaction, activate, overlay, look_disabled)
        })
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
        let name = self
            .accessible
            .clone()
            .unwrap_or_else(|| accessible_name(&self.label));
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
            .tone_scope(self.resolved_tone())
            .stateful_style(variant, user_overlay, move || {
                reactive_style_state(interaction.disabled(look_disabled), look_disabled)
            })
            .seal_style();
        let node = press(
            node,
            interaction,
            disabled,
            Press::Activate(activate),
            RELEASE_ENTER_SPACE,
        );
        let node = match self.pending {
            Some(pending) => node.child(self.pending_content(pending)),
            None if self.square => node.child(self.square_content()),
            None => self.label_content(node),
        };
        interaction.install(node)
    }
}

builder_impls!([] ButtonBuilder: paint(style_accumulator), widget, states(style_accumulator), node);

mod content;
mod glyph;
mod parts;

pub(crate) use glyph::ButtonGlyph;

use parts::{BUTTON_TRANSITION_ATOMS, ButtonFill, ButtonWidth, button_layout};

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
