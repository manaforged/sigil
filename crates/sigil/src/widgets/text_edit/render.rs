use crate::animation_runtime::presets;
use crate::events::{ModifiersState, NodeEventsExt, PointerEvent};
use crate::geometry::Point;
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, fragment, text};
use crate::layout::snap::CellForm;
use crate::layout::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow, rows};
use crate::layout::{TextStyleCapture, with_text_geometry};
use crate::reactive::{Signal, derived, effect};
use crate::style::{
    BORDER_WIDTH, DISABLED_OPACITY, NodeStyleExt, NumericAtomKind, Px, Radius, StateStyle,
    StyleAtom, StyleOverlay, StyleVariant, TextStyle, Token, TransitionId, space,
};

use crate::layout::Position;
use crate::widgets::interaction::InteractionState;

use super::super::shared::binary_style_state;
use super::handlers::TextEditRuntime;

#[path = "render_selection.rs"]
mod selection;
use selection::selection_layer;

const TEXT_EDIT_TRANSITION: TransitionId = presets::SMOOTH;
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

const DEFAULT_HEIGHT: f32 = 160.0;
const DEFAULT_ROWS: f32 = 5.0;

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
    let placeholder_str = text_content(placeholder);
    let displayed = displayed_text(
        value,
        placeholder.clone(),
        runtime.ime.preedit,
        runtime.caret,
    );
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
            edit_state(interaction, disabled)
        })
        .seal_style()
        .child(edit_inner(displayed, runtime, interaction))
}

pub(super) fn edit_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .fill_width()
        .height(CellForm::new(DEFAULT_HEIGHT, rows(DEFAULT_ROWS).px()).pick())
        .min_size(0.0, 0.0)
        .padding(1.0)
}

fn edit_inner(
    displayed: Signal<String>,
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
        .child(edit_lines(displayed, runtime.text_style));
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
        .layout(edit_inner_layout())
        .size_reactive(runtime.viewport_size)
        .class(Radius::Sm.atom())
        .class(Token::Surface.bg())
        .pointer_down(move |event| pointer_drag_start(event, &down, scroll_y))
        .pointer_move(move |event| pointer_drag_move(event, &moved, scroll_y))
        .pointer_up(move |event| pointer_up(event, interaction))
        .child(body)
}

static CARET_ATOMS: &[StyleAtom] = &[Token::OnSurface.bg()];
const CARET_W: f32 = 1.5;

fn caret_layer(runtime: &TextEditRuntime, interaction: InteractionState) -> NodeBuilder {
    let value = runtime.value;
    let caret = runtime.caret;
    let caret_visible = runtime.caret_visible;
    let style = runtime.text_style;
    let caret_affinity = runtime.caret_affinity;
    let content_size = runtime.content_size;
    let ime = runtime.ime.clone();
    fragment()
        .layout(LayoutStyle::block())
        .children_reactive(move || {
            if !interaction.focus_visible().get() || !caret_visible.get() {
                return Vec::new();
            }
            let (buffer, byte) = ime.composing_caret(&value.get(), caret.get());
            let content_width = content_size.get().0;
            let width = (content_width > 0.0).then_some(content_width);
            let text_style = style.get();
            caret_rect(
                &buffer,
                &text_style,
                width,
                HitPosition {
                    byte_index: byte,
                    affinity: caret_affinity.get(),
                },
            )
            .into_iter()
            .collect()
        })
}

fn caret_rect(
    buffer: &str,
    style: &TextStyle,
    width: Option<f32>,
    position: HitPosition,
) -> Option<NodeBuilder> {
    let pos = with_text_geometry(|engine| engine.caret(buffer, style, width, position))?;
    Some(
        container().hittable(false).layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_left(pos.x)
                .inset_top(pos.top)
                .width(CARET_W)
                .height(pos.height.max(1.0)),
        ),
    )
    .map(|node: NodeBuilder| node.class(CARET_ATOMS))
}

fn edit_inner_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .fill_width()
        .grow(1.0)
        .min_size(0.0, 0.0)
        .overflow_y(Overflow::Hidden)
}

fn edit_lines(displayed: Signal<String>, text_style: Signal<TextStyle>) -> TextBuilder {
    text(displayed)
        .mutate(move |node| {
            let _ = node.attach(TextStyleCapture::new(text_style));
        })
        .hittable(false)
        .layout(LayoutStyle::block().fill_width().min_size(0.0, 0.0))
        .class(Token::OnSurface.text())
}

fn pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    let _ = interaction.release_pointer(event);
}

fn pointer_drag_start(event: &mut PointerEvent, runtime: &TextEditRuntime, scroll_y: Signal<f32>) {
    if runtime.disabled() {
        return;
    }
    let offset = runtime.offset_at_point(event.position.x, event.position.y, scroll_y.peek());
    let mut edit = runtime.edit_mut();
    if event.modifiers.contains(ModifiersState::SHIFT) {
        if edit.selection().is_none() {
            runtime.anchor_affinity.set(runtime.caret_affinity.peek());
        }
        edit.extend_to(offset.byte_index);
    } else {
        edit.place_caret(offset.byte_index);
        runtime.anchor_affinity.set(offset.affinity);
    }
    runtime.commit(&edit);
    runtime.caret_affinity.set(offset.affinity);
    event.capture_pointer();
}

fn pointer_drag_move(event: &PointerEvent, runtime: &TextEditRuntime, scroll_y: Signal<f32>) {
    if !event.is_captured() || runtime.disabled() {
        return;
    }
    let offset = runtime.offset_at_point(event.position.x, event.position.y, scroll_y.peek());
    let mut edit = runtime.edit_mut();
    edit.extend_to(offset.byte_index);
    runtime.commit(&edit);
    runtime.caret_affinity.set(offset.affinity);
}

fn displayed_text(
    value: Signal<String>,
    placeholder: TextContent,
    preedit: Signal<String>,
    caret: Signal<usize>,
) -> Signal<String> {
    derived(move || {
        let current = value.get();
        let composing = preedit.get();
        if !composing.is_empty() {
            splice_preedit(&current, caret.get(), &composing)
        } else if current.is_empty() {
            text_content(&placeholder)
        } else {
            current
        }
    })
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

fn text_content(content: &TextContent) -> String {
    content.resolve()
}

fn accessible_label(placeholder: &str) -> String {
    (!placeholder.is_empty())
        .then_some(placeholder)
        .filter(|text| !text.is_empty())
        .map_or_else(|| String::from("Text editor"), ToOwned::to_owned)
}

fn edit_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> crate::style::StyleState {
    binary_style_state(interaction, disabled)
}

fn edit_variant(chrome: bool) -> StyleVariant {
    const CELL_BASE: &[StyleAtom] = &[Token::SurfaceInset.bg(), StyleAtom::BorderWidth(Px(0))];
    if !chrome {
        return StyleVariant::new(&[], StateStyle::EMPTY);
    }
    StyleVariant::new(
        crate::layout::snap::CellForm::new(EDIT_BASE, CELL_BASE).pick(),
        StateStyle::new(EDIT_REST, EDIT_FOCUS, EDIT_HOVER, EDIT_DOWN, EDIT_DISABLED),
    )
}

impl TextEditRuntime {
    fn scroll_into_view(&self) -> Signal<f32> {
        let scroll_y = self.scroll_y;
        let value = self.value;
        let caret = self.caret;
        let text_style = self.text_style;
        let caret_affinity = self.caret_affinity;
        let content_size = self.content_size;
        let viewport_size = self.viewport_size;
        let (_, padding_y) = self.content_padding;
        effect(move || {
            let buffer = value.get();
            let byte = caret.get().min(buffer.len());
            let content_width = content_size.get().0;
            let width = (content_width > 0.0).then_some(content_width);
            let style = text_style.get();
            let pos = with_text_geometry(|engine| {
                engine.caret(
                    &buffer,
                    &style,
                    width,
                    HitPosition {
                        byte_index: byte,
                        affinity: caret_affinity.get(),
                    },
                )
            })
            .unwrap_or_default();
            let visible = padding_y.mul_add(-2.0, viewport_size.get().1).max(1.0);
            let max_scroll = (content_size.get().1 - visible).max(0.0);
            let cur = scroll_y.peek();
            let next = if pos.top < cur {
                pos.top
            } else if pos.top + pos.height > cur + visible {
                pos.top + pos.height - visible
            } else {
                cur
            };
            let next = next.clamp(0.0, max_scroll);
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
