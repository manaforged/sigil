use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{AccessibleName, TextContent};
use crate::kernel::{NodeBuilder, Role, container};
use crate::reactive::Signal;
use crate::style::{ControlSize, Metrics, Radius, Style, Tone};
use crate::widgets::{Layout, Widget};

use super::layout_ext::LayoutOverrides;
use super::shared::{Activate, accessible_name, caller_style, noop_activate};

/// Builder of a button.
#[must_use]
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
pub fn button(label: impl Into<TextContent>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

/// A button that shows one glyph: text or an `Icon`.
pub fn icon_button(glyph: impl Into<ButtonGlyph>, activate: impl Fn() + 'static) -> ButtonBuilder {
    let mut builder = glyph.into().into_button().ghost().activate(activate);
    builder.square = true;
    builder
}

impl ButtonBuilder {
    /// Creates a button builder.
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
    pub fn accessible(mut self, name: impl Into<AccessibleName>) -> Self {
        self.accessible = Some(name.into());
        self
    }

    /// Sets the pending.
    pub const fn pending(mut self, pending: Signal<bool>) -> Self {
        self.pending = Some(pending);
        self
    }

    /// Applies the primary option.
    pub const fn primary(self) -> Self {
        self.tone(Tone::Primary)
    }

    /// Applies the neutral option.
    pub const fn neutral(self) -> Self {
        self.tone(Tone::Neutral)
    }

    /// Applies the destructive option.
    pub const fn destructive(self) -> Self {
        self.tone(Tone::Error)
    }

    /// Applies the outline option.
    pub const fn outline(self) -> Self {
        self.fill(ButtonFill::Outline)
    }

    /// Sets the icon svg.
    #[cfg(feature = "scene")]
    pub fn icon_svg(mut self, svg: impl Into<std::sync::Arc<str>>) -> Self {
        self.leading = Some(svg.into());
        self
    }

    /// Sets the icon.
    #[cfg(feature = "scene")]
    pub fn icon(self, icon: crate::icons::Icon) -> Self {
        self.icon_svg(icon.svg())
    }

    /// Sets the icon shown after the label, such as a dropdown chevron.
    #[cfg(feature = "scene")]
    pub fn trailing(mut self, icon: crate::icons::Icon) -> Self {
        self.trailing = Some(icon.svg().into());
        self
    }

    /// Sets the hint text shown faint after the label, such as a key name.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.shortcut = Some(hint.into());
        self
    }

    /// Applies the ghost option.
    pub const fn ghost(self) -> Self {
        self.fill(ButtonFill::Ghost)
    }

    /// Sets the tone.
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Applies the solid option.
    pub const fn solid(self) -> Self {
        self.fill(ButtonFill::Solid)
    }

    /// Applies the soft option.
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
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the size.
    pub fn size(mut self, size: ControlSize) -> Self {
        let metrics = Metrics::current();
        self.width = ButtonWidth::Hug;
        self.height = metrics.control_height(size);
        self.padding_x = metrics.control_padding(size);
        self
    }

    /// Sets the activate.
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    fn root(mut self) -> NodeBuilder {
        if self.square {
            self.width = ButtonWidth::Fixed(self.height);
            self.padding_x = 0.0;
        } else if self.layout_overrides.fills_width() {
            self.width = ButtonWidth::Fill;
        }
        let activate = self.activate.take().unwrap_or_else(noop_activate);
        let style = std::mem::take(&mut self.style_accumulator);
        let variant = self.resolved_fill().variant();
        let name = self
            .accessible
            .clone()
            .unwrap_or_else(|| accessible_name(&self.label));
        let node = container()
            .role(Role::Button)
            .label(name)
            .layout(button_layout(self.width, self.height, self.padding_x))
            .class(Radius::Lg.atom());
        let node = self
            .framed(node)
            .class(variant.base())
            .class(BUTTON_TRANSITION_ATOMS)
            .tone_scope(self.resolved_tone())
            .variant(variant)
            .disabled_by(self.disabled)
            .busy_by(self.pending)
            .activate_with(activate);
        let node = match self.pending {
            Some(pending) => node.child(self.pending_content(pending)),
            None if self.square => node.child(self.square_content()),
            None => self.label_content(node),
        };
        caller_style(node, style)
    }
}

builder_impls!([] ButtonBuilder: paint(style_accumulator), widget, states(style_accumulator), node);

mod content;
mod glyph;
mod parts;

pub use glyph::ButtonGlyph;

use parts::{BUTTON_TRANSITION_ATOMS, ButtonFill, ButtonWidth, button_layout};

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
