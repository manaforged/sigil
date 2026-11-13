use crate::animation::Transition;
use crate::layout::{AlignItems, LayoutStyle, Length, Space};
use crate::node::{NodeBuilder, Role, TextContent, container};
use crate::reactive::{Signal, memo, signal};
use crate::style::{BORDER_WIDTH, Color, ControlSize, Metrics, Paint, Radius, Token, Tone, space};
use crate::widgets::{TextState, Widget};

use super::adorn::Adornments;
use super::handlers::TextFieldRuntime;

const MIN_WIDTH: f32 = 96.0;

#[derive(Clone, Copy)]
pub(super) struct FieldInputs {
    pub(super) invalid: Signal<bool>,
    pub(super) width: Option<f32>,
}

pub(super) fn build_base_node(
    inputs: FieldInputs,
    placeholder: &TextContent,
    runtime: TextFieldRuntime,
    adorn: Adornments,
) -> NodeBuilder {
    let placeholder_str = placeholder.resolve();
    let displayed = displayed_text(&runtime, placeholder.clone());
    let anchor = selection_anchor(&runtime.state);
    let focused = runtime.focused;
    let disabled = runtime.disabled;
    let mut node = container()
        .focusable(true)
        .track_focused(focused)
        .capture_text_style(runtime.text_style)
        .role(if runtime.password {
            Role::PasswordInput
        } else {
            Role::TextInput
        })
        .label(accessible_label(placeholder_str.as_str()))
        .invalid(inputs.invalid);
    if let Some(flag) = disabled {
        node = node.disabled(flag);
    }
    if !runtime.password {
        node = node
            .text_value(runtime.state.value())
            .text_cursor(runtime.state.caret(), anchor);
    }
    let inner = container()
        .layout(field_inner_layout(&inputs, &adorn))
        .child(super::selection::text_region(runtime, displayed));
    let node = paint_field_chrome(
        node.placeholder(placeholder_str)
            .layout(field_layout(inputs.width)),
        focused,
        disabled,
        inputs.invalid,
    );
    adorn.wrap(node, inner)
}

fn paint_field_chrome(
    node: NodeBuilder,
    focused: Signal<bool>,
    disabled: Option<Signal<bool>>,
    invalid: Signal<bool>,
) -> NodeBuilder {
    let hovered = signal(false);
    let pressed = signal(false);
    let hover_border = memo(move || {
        !disabled.is_some_and(Signal::get)
            && !invalid.get()
            && hovered.get()
            && !focused.get()
            && !pressed.get()
    });
    let active_border = memo(move || {
        !disabled.is_some_and(Signal::get) && !invalid.get() && (focused.get() || pressed.get())
    });
    let node = node
        .track_hovered(hovered)
        .track_pressed(pressed)
        .radius(Radius::Md)
        .background(Token::Field)
        .border_width(BORDER_WIDTH)
        .border_color(Tone::Neutral.step(7))
        .transition(Transition::smooth())
        .paint_when(hover_border, |s| s.border_color(Tone::Neutral.step(8)))
        .paint_when(active_border, |s| s.border_color(Token::Accent));
    match disabled {
        Some(flag) => node
            .paint_when(memo(move || !flag.get() && invalid.get()), |s| {
                s.border_color(Color::rgb(211, 47, 47))
            })
            .paint_when(flag, |s| s.border_color(Tone::Neutral.step(6)).opacity(0.5)),
        None => node.paint_when(invalid, |s| s.border_color(Color::rgb(211, 47, 47))),
    }
}

fn selection_anchor(state: &TextState) -> Signal<Option<usize>> {
    let state = state.clone();
    memo(move || {
        let caret = state.caret().get();
        state.selection().map(|range| {
            if range.start == caret {
                range.end
            } else {
                range.start
            }
        })
    })
}

pub(super) fn field_layout(width: Option<f32>) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .min_size(
            width.map_or(MIN_WIDTH, |_| 0.0),
            Metrics::current().control_height(ControlSize::Default),
        )
        .align_items(AlignItems::Stretch)
}

fn field_inner_layout(inputs: &FieldInputs, adorn: &Adornments) -> LayoutStyle {
    let width = inputs.width;
    let leading = adorn.leading.is_some();
    let trailing = adorn.trailing.is_some() || adorn.clear.is_some();
    let metrics = Metrics::current();
    let inset = if metrics.is_grid() {
        metrics.icon_size()
    } else {
        1.0
    };
    let v_pad = if metrics.is_grid() { 0.0 } else { space(1) };
    let adorned = leading || trailing;
    let base = match width {
        _ if adorned => LayoutStyle::flex_row()
            .grow(1.0)
            .shrink(1.0)
            .min_size(0.0, Length::Auto),
        Some(w) => LayoutStyle::flex_row().width(2.0_f32.mul_add(-inset, w).max(0.0)),
        None => LayoutStyle::flex_row().fill_width().grow(1.0),
    };
    let edge = |side: bool| if side { space(2) } else { space(3) };
    base.align_items(AlignItems::Center)
        .padding_left(Space::px(edge(leading)))
        .padding_right(Space::px(edge(trailing)))
        .padding_y(Space::px(v_pad))
}

fn displayed_text(runtime: &TextFieldRuntime, placeholder: TextContent) -> Signal<String> {
    let value = runtime.state.value();
    let preedit = runtime.preedit;
    let caret = runtime.state.caret();
    let password = runtime.password;
    let reveal = runtime.reveal;
    memo(move || {
        let current = value.get();
        let composing = preedit.get();
        let hiding = super::mask::hiding(password, reveal);
        let shown = if !composing.is_empty() {
            TextState::splice_preedit(&current, caret.get(), &composing)
        } else if current.is_empty() {
            return placeholder.resolve();
        } else {
            current
        };
        if hiding {
            super::mask::mask_chars(&shown)
        } else {
            shown
        }
    })
}

fn accessible_label(placeholder: &str) -> String {
    if placeholder.is_empty() {
        String::from("Text field")
    } else {
        placeholder.to_owned()
    }
}
