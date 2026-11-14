use std::fmt;
use std::rc::Rc;

use sigil::animation::Transition;
use sigil::layout::LayoutStyle;
use sigil::node::{Cx, NodeId};
use sigil::prelude::*;
use sigil::style::{BORDER_WIDTH, ColorValue, Elevation, Metrics};
use sigil::widgets::{Framed, WidgetFrame};

#[derive(Clone, Copy)]
enum ButtonVariant {
    Soft,
    Solid,
    Ghost,
}

/// External reimplementation of `button` using only public Layer 1 and Layer 2 APIs.
#[must_use]
pub struct CustomButtonBuilder {
    label: TextContent,
    variant: ButtonVariant,
    disabled: Option<Signal<bool>>,
    pending: Option<Signal<bool>>,
    activate: Option<Rc<dyn Fn()>>,
    frame: WidgetFrame,
}

/// Creates a custom button with `label` built from public Layer 2 primitives.
pub fn custom_button(label: impl Into<TextContent>) -> CustomButtonBuilder {
    CustomButtonBuilder {
        label: label.into(),
        variant: ButtonVariant::Soft,
        disabled: None,
        pending: None,
        activate: None,
        frame: WidgetFrame::new(),
    }
}

impl fmt::Debug for CustomButtonBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomButtonBuilder")
            .finish_non_exhaustive()
    }
}

impl Framed for CustomButtonBuilder {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for CustomButtonBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

impl CustomButtonBuilder {
    /// Applies the solid primary accent appearance.
    pub const fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Solid;
        self
    }

    /// Applies the transparent ghost appearance.
    pub const fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    /// Disables the button while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Marks the button busy and shows a spinner while `pending` is true.
    pub const fn pending(mut self, pending: Signal<bool>) -> Self {
        self.pending = Some(pending);
        self
    }

    /// Runs `activate` on click, Enter, or Space.
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    fn root(mut self) -> NodeBuilder {
        let m = Metrics::current();
        let p = button_palette(self.variant);
        let activate = self.activate.take().unwrap_or_else(|| Rc::new(|| {}));
        let layout = LayoutStyle::flex_row()
            .height(m.control_height(ControlSize::Default))
            .padding_x(Space::raw(
                m.control_padding(ControlSize::Default),
                "control padding",
            ))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center);
        let layout = if self.frame.fills_width() {
            layout
        } else {
            layout.shrink(0.0)
        };
        let disabled_bg = p.disabled_bg;
        let mut node = container()
            .role(Role::Button)
            .label(AccessibleName::from(&self.label))
            .layout(layout)
            .radius(Radius::Lg)
            .transition(Transition::smooth())
            .border_width(BORDER_WIDTH)
            .border_color(p.border)
            .text_color(p.fg)
            .focus(move |s| s.background(p.hover_bg))
            .hover(move |s| s.background(p.hover_bg))
            .pressed(move |s| s.background(p.pressed_bg))
            .disabled_paint(move |s| {
                let s = s.opacity(0.5);
                match disabled_bg {
                    Some(bg) => s.background(bg),
                    None => s,
                }
            })
            .activate(move || activate());
        if let Some(bg) = p.rest_bg {
            node = node.background(bg);
        }
        if let Some(el) = p.shadow {
            node = node.shadow(el);
        }
        if let Some(flag) = self.disabled {
            node = node.disabled(flag);
        }
        let out = attach_button_content(node, self.label, self.pending, self.variant, m);
        self.frame.finish(out)
    }
}

#[derive(Clone, Copy)]
struct ButtonPalette {
    rest_bg: Option<Token>,
    hover_bg: Token,
    pressed_bg: Token,
    disabled_bg: Option<Token>,
    border: ColorValue,
    fg: ColorValue,
    shadow: Option<Elevation>,
}

const fn button_palette(variant: ButtonVariant) -> ButtonPalette {
    let (n, p) = (Tone::Neutral, Tone::Primary);
    match variant {
        ButtonVariant::Solid => ButtonPalette {
            rest_bg: Some(p.step(9)),
            hover_bg: p.step(10),
            pressed_bg: p.step(11),
            disabled_bg: Some(p.step(8)),
            border: ColorValue::Color(Color::TRANSPARENT),
            fg: ColorValue::Token(Token::OnSolid),
            shadow: Some(Elevation::Sm),
        },
        ButtonVariant::Soft => ButtonPalette {
            rest_bg: Some(n.step(4)),
            hover_bg: n.step(5),
            pressed_bg: n.step(6),
            disabled_bg: Some(n.step(2)),
            border: ColorValue::Token(n.step(6)),
            fg: ColorValue::Token(n.step(11)),
            shadow: None,
        },
        ButtonVariant::Ghost => ButtonPalette {
            rest_bg: None,
            hover_bg: n.step(4),
            pressed_bg: n.step(5),
            disabled_bg: None,
            border: ColorValue::Color(Color::TRANSPARENT),
            fg: ColorValue::Token(n.step(11)),
            shadow: None,
        },
    }
}

fn label_paint<P: Paint>(target: P, variant: ButtonVariant) -> P {
    let solid_grid = matches!(variant, ButtonVariant::Solid) && Metrics::current().is_grid();
    let target = target
        .text_color(button_palette(variant).fg)
        .font_weight(if solid_grid { 700 } else { 500 });
    if solid_grid {
        target
    } else {
        target.letter_spacing(-0.01)
    }
}

fn attach_button_content(
    node: NodeBuilder,
    label: TextContent,
    pending: Option<Signal<bool>>,
    variant: ButtonVariant,
    m: Metrics,
) -> NodeBuilder {
    match pending {
        Some(busy) => {
            let row = LayoutStyle::flex_row()
                .align_items(AlignItems::Center)
                .gap(Space::raw(
                    m.control_gap(ControlSize::Compact),
                    "control gap",
                ));
            node.busy(busy).child(
                label_paint(container().hittable(false).layout(row), variant).children_reactive(
                    move || {
                        let mut parts = Vec::with_capacity(2);
                        if busy.get() {
                            parts.push(spinner().hittable(false));
                        }
                        parts.push(
                            container().child(label_paint(text(label.clone()).nowrap(), variant)),
                        );
                        parts
                    },
                ),
            )
        }
        None => node.child(label_paint(text(label).nowrap(), variant)),
    }
}
