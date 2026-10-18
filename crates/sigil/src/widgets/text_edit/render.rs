use crate::animation_runtime::presets;
use crate::events::{ModifiersState, NodeEventsExt, PointerEvent};
use crate::geometry::Point;
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, fragment, text};
use crate::layout::snap::CellForm;
use crate::layout::{
    CaretGeometry, LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow, Position,
    TextStyleCapture, rows, with_text_geometry,
};
use crate::reactive::{Signal, derived, effect};
use crate::style::TransitionId;
use crate::style::{
    BORDER_WIDTH, DISABLED_OPACITY, NodeStyleExt, NumericAtomKind, Px, Radius, StateStyle,
    StyleAtom, StyleOverlay, StyleVariant, Token, space,
};

use crate::widgets::interaction::InteractionState;

use super::super::shared::binary_style_state;
use super::handlers::TextEditRuntime;

#[path = "render_selection.rs"]
mod selection;
use selection::selection_layer;

const TEXT_EDIT_TRANSITION: TransitionId = presets::SMOOTH;
const DEFAULT_ROWS: f32 = 5.0;
static TEXT_EDIT_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TEXT_EDIT_TRANSITION)];

static EDIT_BASE: &[StyleAtom] = &[
    Token::SurfaceInset.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
];
static EDIT_REST: &[StyleAtom] = &[StyleAtom::BorderToneStep(9)];
static EDIT_FOCUS: &[StyleAtom] = &[Token::Accent.border()];
static EDIT_HOVER: &[StyleAtom] = &[StyleAtom::BorderToneStep(10)];
static EDIT_DOWN: &[StyleAtom] = &[Token::Accent.border()];
static EDIT_DISABLED: &[StyleAtom] = &[
    StyleAtom::BorderToneStep(8),
    StyleAtom::Opacity(DISABLED_OPACITY),
];

pub(super) const fn content_padding() -> (f32, f32) {
    (space(3), space(2))
}

pub(super) fn build_base_node(
    runtime: &TextEditRuntime,
    placeholder: &TextContent,
    interaction: InteractionState,
    user_overlay: StyleOverlay,
    chrome: bool,
) -> NodeBuilder {
    let value = runtime.value;
    let disabled = runtime.disabled;
    let placeholder_str = placeholder.resolve();
    let lines = edit_lines(runtime, placeholder.clone());
    container()
        .mutate({
            let text_style = runtime.text_style;
            move |node| {
                let _ = node.attach(TextStyleCapture::new(text_style));
            }
        })
        .role(Role::TextInput)
        .label(accessible_label(placeholder_str.as_str()))
        .text_value(value)
        .text_cursor(runtime.caret, runtime.selection)
        .placeholder(placeholder_str)
        .layout(edit_layout())
        .class(Radius::Md.atom())
        .class(TEXT_EDIT_ANIM_BASE)
        .stateful_style(edit_variant(chrome), user_overlay, move || {
            binary_style_state(interaction, disabled)
        })
        .seal_style()
        .child(edit_inner(lines, runtime, interaction))
}

pub(super) fn edit_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .fill_width()
        .height(CellForm::new(160.0, rows(DEFAULT_ROWS).px()).pick())
        .min_size(0.0, 0.0)
        .padding(1.0)
}

fn edit_inner(
    lines: TextBuilder,
    runtime: &TextEditRuntime,
    interaction: InteractionState,
) -> NodeBuilder {
    let down = runtime.clone();
    let moved = runtime.clone();
    let scroll_y = runtime.scroll_into_view();
    let (padding_x, padding_y) = runtime.content_padding;
    let offset = derived(move || (0.0, padding_y - scroll_y.get()));
    let text_box = container()
        .hittable(false)
        .layout(LayoutStyle::block().min_size(0.0, 0.0))
        .size_reactive(runtime.content_size)
        .position_reactive(offset)
        .child(selection_layer(runtime))
        .child(caret_layer(runtime, interaction))
        .child(lines);
    let content_size = runtime.content_size;
    let extent = derived(move || padding_y.mul_add(2.0, content_size.get().1));
    let body = container()
        .layout(
            LayoutStyle::block()
                .fill_width()
                .min_size(0.0, 0.0)
                .padding_x(padding_x),
        )
        .animate(NumericAtomKind::Height, extent)
        .child(text_box);
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .overflow_y(Overflow::Hidden),
        )
        .size_reactive(runtime.viewport_size)
        .class(Radius::Sm.atom())
        .class(Token::Surface.bg())
        .pointer_down(move |event| pointer_drag(event, &down, scroll_y, true))
        .pointer_move(move |event| pointer_drag(event, &moved, scroll_y, false))
        .pointer_up(move |event| {
            let _ = interaction.release_pointer(event);
        })
        .child(body)
}

static CARET_ATOMS: &[StyleAtom] = &[Token::OnSurface.bg()];
const CARET_W: f32 = 1.5;

fn caret_layer(runtime: &TextEditRuntime, interaction: InteractionState) -> NodeBuilder {
    let value = runtime.value;
    let caret = runtime.caret;
    let caret_visible = runtime.caret_visible;
    let runtime = runtime.clone();
    fragment()
        .layout(LayoutStyle::block())
        .children_reactive(move || {
            if !interaction.focus_visible().get() || !caret_visible.get() {
                return Vec::new();
            }
            let (buffer, byte) = runtime.ime.composing_caret(&value.get(), caret.get());
            let Some(pos) = runtime.caret_geometry(&buffer, byte) else {
                return Vec::new();
            };
            let height = pos.height.max(1.0);
            vec![overlay_rect(pos.x, pos.top, CARET_W, height).class(CARET_ATOMS)]
        })
}

pub(in crate::widgets) fn overlay_rect(x: f32, top: f32, width: f32, height: f32) -> NodeBuilder {
    container().hittable(false).layout(
        LayoutStyle::block()
            .position(Position::Absolute)
            .inset_left(x)
            .inset_top(top)
            .width(width)
            .height(height),
    )
}

fn pointer_drag(
    event: &mut PointerEvent,
    runtime: &TextEditRuntime,
    scroll_y: Signal<f32>,
    down: bool,
) {
    if runtime.disabled() || !(down || event.is_captured()) {
        return;
    }
    let offset = runtime.offset_at_point(event.position.x, event.position.y, scroll_y.peek());
    let mut edit = runtime.edit_mut();
    let extend = !down || event.modifiers.contains(ModifiersState::SHIFT);
    if down && !extend {
        runtime.anchor_affinity.set(offset.affinity);
    } else if down && edit.selection().is_none() {
        runtime.anchor_affinity.set(runtime.caret_affinity.peek());
    }
    edit.jump_to(offset.byte_index, extend);
    runtime.commit(&edit);
    runtime.caret_affinity.set(offset.affinity);
    if down {
        event.capture_pointer();
    }
}

fn edit_lines(runtime: &TextEditRuntime, placeholder: TextContent) -> TextBuilder {
    let (value, preedit, caret) = (runtime.value, runtime.ime.preedit, runtime.caret);
    let text_style = runtime.text_style;
    text(derived(move || {
        let current = value.get();
        let composing = preedit.get();
        if !composing.is_empty() {
            splice_preedit(&current, caret.get(), &composing)
        } else if current.is_empty() {
            placeholder.resolve()
        } else {
            current
        }
    }))
    .mutate(move |node| {
        let _ = node.attach(TextStyleCapture::new(text_style));
    })
    .hittable(false)
    .layout(LayoutStyle::block().fill_width().min_size(0.0, 0.0))
    .class(Token::OnSurface.text())
}

pub(crate) fn splice_preedit(value: &str, caret: usize, preedit: &str) -> String {
    let caret = floor_char_boundary(value, caret);
    let mut out = String::with_capacity(value.len() + preedit.len());
    out.push_str(&value[..caret]);
    out.push_str(preedit);
    out.push_str(&value[caret..]);
    out
}

pub(crate) fn floor_char_boundary(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

fn accessible_label(placeholder: &str) -> String {
    if placeholder.is_empty() {
        String::from("Text editor")
    } else {
        placeholder.to_owned()
    }
}

fn edit_variant(chrome: bool) -> StyleVariant {
    const CELL_BASE: &[StyleAtom] = &[Token::SurfaceInset.bg(), StyleAtom::BorderWidth(Px(0))];
    if !chrome {
        return StyleVariant::new(&[], StateStyle::EMPTY);
    }
    StyleVariant::new(
        CellForm::new(EDIT_BASE, CELL_BASE).pick(),
        StateStyle::new(EDIT_REST, EDIT_FOCUS, EDIT_HOVER, EDIT_DOWN, EDIT_DISABLED),
    )
}

impl TextEditRuntime {
    fn caret_geometry(&self, buffer: &str, byte: usize) -> Option<CaretGeometry> {
        let style = self.text_style.get();
        let position = HitPosition {
            byte_index: byte,
            affinity: self.caret_affinity.get(),
        };
        let width = self.geometry_width();
        with_text_geometry(|engine| engine.caret(buffer, &style, width, position))
    }

    fn scroll_into_view(&self) -> Signal<f32> {
        let scroll_y = self.scroll_y;
        let viewport_size = self.viewport_size;
        let (_, padding_y) = self.content_padding;
        let runtime = self.clone();
        effect(move || {
            let buffer = runtime.value.get();
            let byte = runtime.caret.get().min(buffer.len());
            let pos = runtime.caret_geometry(&buffer, byte).unwrap_or_default();
            let window = viewport_size.get().1;
            let visible = padding_y.mul_add(-2.0, window);
            let max_scroll =
                (padding_y.mul_add(2.0, runtime.content_size.get().1) - window).max(0.0);
            let cur = scroll_y.peek();
            let span = if pos.height > visible {
                ((pos.top + padding_y, pos.height), window)
            } else {
                ((pos.top, pos.height), visible)
            };
            let next = crate::widgets::scroll::nearest(span.0, cur, span.1).clamp(0.0, max_scroll);
            if (next - cur).abs() > f32::EPSILON {
                scroll_y.set(next);
            }
        });
        scroll_y
    }

    fn offset_at_point(&self, x: f32, y: f32, scroll_y: f32) -> HitPosition {
        let buffer = self.value.peek();
        let (padding_x, padding_y) = self.content_padding;
        let content_y = y - padding_y + scroll_y;
        let content_x = x - padding_x;
        let style = self.text_style.peek();
        let width = self.geometry_width();
        with_text_geometry(|engine| {
            engine.hit(&buffer, &style, width, Point::new(content_x, content_y))
        })
        .unwrap_or(HitPosition {
            byte_index: 0,
            affinity: TextAffinity::Downstream,
        })
    }
}
