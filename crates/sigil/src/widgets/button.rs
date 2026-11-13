use crate::reactive::Reactive;
use std::rc::Rc;

use crate::kernel::{AccessibleName, TextContent};
use crate::kernel::{NodeBuilder, Role, container};
use crate::reactive::Signal;
use crate::style::{ControlSize, Metrics, Radius, Style, Tone};
use crate::widgets::Widget;

use super::icon::Glyph;
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

/// A push button. A text label makes a text button; an `Icon` makes a square ghost button named for the icon.
pub fn button(label: impl Into<Glyph>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

impl ButtonBuilder {
    /// Creates a button from a text label or an icon; an icon makes a square ghost button.
    pub(crate) fn new(label: impl Into<Glyph>) -> Self {
        match label.into() {
            Glyph::Text(text) => Self::labelled(text),
            #[cfg(feature = "scene")]
            Glyph::Icon(icon) => Self::labelled(icon.name().into())
                .icon(icon)
                .accessible(icon.name())
                .ghost()
                .square(),
            #[cfg(feature = "scene")]
            Glyph::Svg(source) => Self::labelled("".into()).icon_svg(source).ghost().square(),
        }
    }

    fn labelled(label: TextContent) -> Self {
        let metrics = Metrics::current();
        Self {
            label,
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

    /// Makes the button as wide as it is high, for a one-glyph label.
    pub const fn square(mut self) -> Self {
        self.square = true;
        self
    }

    /// Sets the name that screen readers announce in place of the label.
    pub fn accessible(mut self, name: impl Into<AccessibleName>) -> Self {
        self.accessible = Some(name.into());
        self
    }

    /// Shows a spinner before the label and disables the button while `pending` is true.
    pub const fn pending(mut self, pending: Signal<bool>) -> Self {
        self.pending = Some(pending);
        self
    }

    /// Uses `Tone::Primary`, which fills the button solid unless a fill is set.
    pub const fn primary(self) -> Self {
        self.tone(Tone::Primary)
    }

    /// Uses `Tone::Neutral`, which is the default tone.
    pub const fn neutral(self) -> Self {
        self.tone(Tone::Neutral)
    }

    /// Uses `Tone::Error`, which fills the button solid unless a fill is set.
    pub const fn destructive(self) -> Self {
        self.tone(Tone::Error)
    }

    /// Draws the button as a border with no fill.
    pub const fn outline(self) -> Self {
        self.fill(ButtonFill::Outline)
    }

    /// Shows the SVG source `svg` as an icon before the label.
    #[cfg(feature = "scene")]
    pub fn icon_svg(mut self, svg: impl Into<std::sync::Arc<str>>) -> Self {
        self.leading = Some(svg.into());
        self
    }

    /// Shows the bundled `icon` before the label.
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

    /// Draws the button with no border and no fill until it has hover, focus, or a press.
    pub const fn ghost(self) -> Self {
        self.fill(ButtonFill::Ghost)
    }

    /// Sets the color tone; `Primary` and `Error` fill solid and other tones fill soft by default.
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Fills the button with the solid look of its tone and adds a small shadow.
    pub const fn solid(self) -> Self {
        self.fill(ButtonFill::Solid)
    }

    /// Fills the button with a light tint of its tone and draws a border.
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

    /// Disables the button while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the height and horizontal padding from `size`; the default is `ControlSize::Default`.
    pub fn size(mut self, size: ControlSize) -> Self {
        let metrics = Metrics::current();
        self.width = ButtonWidth::Hug;
        self.height = metrics.control_height(size);
        self.padding_x = metrics.control_padding(size);
        self
    }

    /// Runs `activate` when the user activates the button.
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
mod parts;

use parts::{BUTTON_TRANSITION_ATOMS, ButtonFill, ButtonWidth, button_layout};

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
