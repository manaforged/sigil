use sigil_animation_runtime::presets;
use sigil_events::prelude::{NodeEventsExt, PointerEvent};
use sigil_kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, fragment, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow};
use sigil_reactive::{Signal, derived, derived_eq, effect, signal};
use sigil_style::{
    NodeStyleExt, Px, StateStyle, StyleAtom, StyleOverlay, StyleVariant, TransitionId,
};

use sigil_layout::prelude::Position;
use sigil_layout::with_text_geometry;
use sigil_style::TextStyle;

use crate::interaction::InteractionState;

use super::super::shared::binary_style_state;
use super::super::transition_seam::install_transition_animators;
use super::handlers::TextEditRuntime;

const TEXT_EDIT_TRANSITION: TransitionId = presets::SMOOTH;
const TEXT_EDIT_COMPONENT_ID: u64 = 0x7E47_F100;
static TEXT_EDIT_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(TEXT_EDIT_TRANSITION)];

static EDIT_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-inset"),
    StyleAtom::BorderWidth(Px(1)),
];
static EDIT_REST: &[StyleAtom] = &[StyleAtom::BorderToneStep(9)];
static EDIT_FOCUS: &[StyleAtom] = &[StyleAtom::BorderThemeRef("accent")];
static EDIT_HOVER: &[StyleAtom] = &[StyleAtom::BorderToneStep(10)];
static EDIT_DOWN: &[StyleAtom] = &[StyleAtom::BorderThemeRef("accent")];
static EDIT_DISABLED: &[StyleAtom] = &[StyleAtom::BorderToneStep(8), StyleAtom::Opacity(5_000)];

const MAX_LINES: usize = 2000;

pub(super) const DEFAULT_HEIGHT: f32 = 160.0;
const BORDER_PX: f32 = 1.0;
pub(super) const CONTENT_PADDING_X: f32 = 12.0;
pub(super) const CONTENT_PADDING_Y: f32 = 8.0;

pub(super) fn build_base_node(
    runtime: &TextEditRuntime,
    placeholder: &TextContent,
    interaction: InteractionState,
    user_overlay: StyleOverlay,
    frame_h: f32,
) -> NodeBuilder {
    let value = runtime.value;
    let disabled = runtime.disabled;
    let placeholder_str = text_content_untracked(placeholder);
    let displayed = displayed_text(
        value,
        interaction.focus_visible(),
        placeholder.clone(),
        runtime.ime.preedit,
        runtime.caret,
    );
    let state_style = edit_state_style();
    let node = container()
        .role(Role::TextInput)
        .label(accessible_label(placeholder_str.as_str()))
        .text_value(value)
        .placeholder(placeholder_str)
        .layout(edit_layout())
        .class("rounded-md")
        .class(TEXT_EDIT_ANIM_BASE)
        .stateful_style(edit_variant(), user_overlay, move || {
            edit_state(interaction, disabled)
        })
        .child(edit_inner(displayed, runtime, interaction, frame_h));
    install_transition_animators(
        node,
        TEXT_EDIT_ANIM_BASE,
        state_style,
        TEXT_EDIT_COMPONENT_ID,
        interaction,
    )
}

const fn edit_state_style() -> StateStyle {
    StateStyle::new(EDIT_REST, EDIT_FOCUS, EDIT_HOVER, EDIT_DOWN, EDIT_DISABLED)
}

pub(super) fn edit_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .fill_width()
        .height(DEFAULT_HEIGHT)
        .min_size(0.0, 0.0)
}

fn edit_inner(
    displayed: Signal<String>,
    runtime: &TextEditRuntime,
    interaction: InteractionState,
    frame_h: f32,
) -> NodeBuilder {
    let down = runtime.clone();
    let moved = runtime.clone();
    let scroll_y = scroll_into_view(runtime, frame_h);
    let content_pos = derived(move || (CONTENT_PADDING_X, CONTENT_PADDING_Y - scroll_y.get()));
    let content = container()
        .hittable(false)
        .layout(LayoutStyle::block())
        .position_reactive(content_pos)
        .child(selection_layer(runtime))
        .child(edit_lines(displayed))
        .child(caret_layer(runtime, interaction));
    container()
        .layout(edit_inner_layout())
        .class("rounded-sm")
        .class("bg-surface")
        .pointer_down(move |event| pointer_drag_start(event, &down, scroll_y))
        .pointer_move(move |event| pointer_drag_move(event, &moved, scroll_y))
        .pointer_up(move |event| pointer_up(event, interaction))
        .child(content)
}

fn visible_h(frame_h: f32) -> f32 {
    (frame_h - BORDER_PX - BORDER_PX - CONTENT_PADDING_Y - CONTENT_PADDING_Y).max(1.0)
}

fn scroll_into_view(runtime: &TextEditRuntime, frame_h: f32) -> Signal<f32> {
    let scroll_y = signal(0.0_f32);
    let value = runtime.value;
    let caret = runtime.caret;
    let visible = visible_h(frame_h);
    effect(move || {
        let buffer = value.get();
        let byte = caret.get().min(buffer.len());
        let line = caret_line_col(&buffer, byte).0;
        let line_h = line_height();
        #[allow(
            clippy::cast_precision_loss,
            reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
        )]
        let top = line as f32 * line_h;
        let cur = scroll_y.peek();
        let next = if top < cur {
            top
        } else if top + line_h > cur + visible {
            top + line_h - visible
        } else {
            cur
        };
        if (next - cur).abs() > f32::EPSILON {
            scroll_y.set(next.max(0.0));
        }
    });
    scroll_y
}

static SELECTION_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundColor(
    sigil_style::ColorAtom::Rgba8(sigil_style::Rgba8 {
        r: 56,
        g: 118,
        b: 224,
        a: 130,
    }),
)];
const NEWLINE_SEL_W: f32 = 6.0;

fn selection_layer(runtime: &TextEditRuntime) -> NodeBuilder {
    let value = runtime.value;
    let caret = runtime.caret;
    let selection = runtime.selection;
    fragment()
        .layout(LayoutStyle::block())
        .children_reactive(move || {
            let buffer = value.get();
            let Some(anchor) = selection.get() else {
                return Vec::new();
            };
            let caret = caret.get().min(buffer.len());
            let anchor = anchor.min(buffer.len());
            let (lo, hi) = (anchor.min(caret), anchor.max(caret));
            selection_rects(&buffer, lo, hi)
        })
}

fn selection_rects(buffer: &str, lo: usize, hi: usize) -> Vec<NodeBuilder> {
    if lo >= hi {
        return Vec::new();
    }
    let (lo_line, lo_col) = caret_line_col(buffer, lo);
    let (hi_line, hi_col) = caret_line_col(buffer, hi);
    let line_h = line_height();
    (lo_line..=hi_line)
        .filter_map(|line| {
            let start_col = if line == lo_line { lo_col } else { 0 };
            let end_col = if line == hi_line { hi_col } else { usize::MAX };
            line_highlight(buffer, line, (start_col, end_col), hi_line, line_h)
        })
        .collect()
}

fn line_highlight(
    buffer: &str,
    line: usize,
    cols: (usize, usize),
    hi_line: usize,
    line_h: f32,
) -> Option<NodeBuilder> {
    let (start_col, end_col) = cols;
    let line_text = nth_line(buffer, line);
    let (x0, x1) = with_text_geometry(|engine| {
        let style = TextStyle::default();
        let a = engine
            .caret(line_text, &style, start_col.min(line_text.len()))
            .x;
        let b = engine
            .caret(line_text, &style, end_col.min(line_text.len()))
            .x;
        (a, b)
    })?;
    let min_w = if line < hi_line { NEWLINE_SEL_W } else { 0.0 };
    let width = (x1 - x0).max(min_w);
    if width <= 0.0 {
        return None;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let y = line as f32 * line_h;
    Some(
        container()
            .hittable(false)
            .layout(
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_left(x0)
                    .inset_top(y)
                    .width(width)
                    .height(line_h),
            )
            .class(SELECTION_ATOMS),
    )
}

static CARET_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
const CARET_W: f32 = 1.5;
const FALLBACK_LINE_H: f32 = 16.0;

fn caret_layer(runtime: &TextEditRuntime, interaction: InteractionState) -> NodeBuilder {
    let value = runtime.value;
    let caret = runtime.caret;
    let caret_visible = runtime.caret_visible;
    fragment()
        .layout(LayoutStyle::block())
        .children_reactive(move || {
            if !interaction.focus_visible().get() || !caret_visible.get() {
                return Vec::new();
            }
            let buffer = value.get();
            let byte = caret.get().min(buffer.len());
            caret_rect(&buffer, byte).into_iter().collect()
        })
}

fn caret_rect(buffer: &str, byte: usize) -> Option<NodeBuilder> {
    let (line, col_byte) = caret_line_col(buffer, byte);
    let line_text = nth_line(buffer, line);
    let line_h = line_height();
    let x = with_text_geometry(|engine| {
        engine
            .caret(
                line_text,
                &TextStyle::default(),
                col_byte.min(line_text.len()),
            )
            .x
    })?;
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let y = line as f32 * line_h;
    Some(
        container().hittable(false).layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_left(x)
                .inset_top(y)
                .width(CARET_W)
                .height(line_h),
        ),
    )
    .map(|node: NodeBuilder| node.class(CARET_ATOMS))
}

fn line_height() -> f32 {
    with_text_geometry(|engine| engine.caret("", &TextStyle::default(), 0).height)
        .filter(|h| *h > 0.0)
        .unwrap_or(FALLBACK_LINE_H)
}

fn caret_line_col(buffer: &str, caret: usize) -> (usize, usize) {
    let caret = caret.min(buffer.len());
    let before = &buffer[..caret];
    let line = before.bytes().filter(|&b| b == b'\n').count();
    let col = before.rfind('\n').map_or(caret, |nl| caret - (nl + 1));
    (line, col)
}

fn nth_line(buffer: &str, n: usize) -> &str {
    buffer.split('\n').nth(n).unwrap_or("")
}

fn edit_inner_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .fill()
        .min_size(0.0, 0.0)
        .overflow_y(Overflow::Hidden)
}

fn edit_lines(displayed: Signal<String>) -> NodeBuilder {
    let lines_signal = derived_eq(move || split_lines(displayed.get().as_str()));
    fragment()
        .layout(LayoutStyle::flex_column().fill_width().min_size(0.0, 0.0))
        .children_reactive(move || line_nodes(lines_signal.get()))
}

fn line_nodes(lines: Vec<String>) -> Vec<TextBuilder> {
    if lines.is_empty() {
        return vec![text(String::new()).class("text-on-surface")];
    }
    let total = lines.len();
    let mut nodes: Vec<TextBuilder> = lines
        .into_iter()
        .take(MAX_LINES)
        .map(|line| {
            text(line)
                .hittable(false)
                .layout(LayoutStyle::block().fill_width().min_size(0.0, 0.0))
                .class("text-on-surface")
        })
        .collect();
    if let Some(notice) = truncation_notice(total) {
        nodes.push(
            text(notice)
                .layout(LayoutStyle::block().fill_width().min_size(0.0, 0.0))
                .class("text-muted-foreground"),
        );
    }
    nodes
}

fn truncation_notice(total: usize) -> Option<String> {
    total
        .checked_sub(MAX_LINES)
        .filter(|&hidden| hidden > 0)
        .map(|hidden| format!("… {hidden} more line(s) not shown"))
}

fn split_lines(source: &str) -> Vec<String> {
    if source.is_empty() {
        return Vec::new();
    }
    source.split('\n').map(ToOwned::to_owned).collect()
}

fn pointer_up(event: &mut PointerEvent, interaction: InteractionState) {
    let _ = interaction.release_pointer(event);
}

fn pointer_drag_start(event: &mut PointerEvent, runtime: &TextEditRuntime, scroll_y: Signal<f32>) {
    let buffer = runtime.value.peek();
    let offset = offset_at_point(&buffer, event.position.x, event.position.y, scroll_y.peek());
    let mut edit = runtime.edit_mut();
    edit.place_caret(offset);
    super::handlers::commit(runtime, &edit);
    event.capture_pointer();
}

fn pointer_drag_move(event: &PointerEvent, runtime: &TextEditRuntime, scroll_y: Signal<f32>) {
    if !event.is_captured() {
        return;
    }
    let buffer = runtime.value.peek();
    let offset = offset_at_point(&buffer, event.position.x, event.position.y, scroll_y.peek());
    let mut edit = runtime.edit_mut();
    edit.extend_to(offset);
    super::handlers::commit(runtime, &edit);
}

fn offset_at_point(buffer: &str, x: f32, y: f32, scroll_y: f32) -> usize {
    let line_h = line_height();
    let content_y = (y - CONTENT_PADDING_Y + scroll_y).max(0.0);
    let content_x = (x - CONTENT_PADDING_X).max(0.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let raw_line = (content_y / line_h) as usize;
    let last_line = buffer.bytes().filter(|&b| b == b'\n').count();
    let line = raw_line.min(last_line);
    let line_text = nth_line(buffer, line);
    let col = with_text_geometry(|engine| engine.hit(line_text, &TextStyle::default(), content_x))
        .unwrap_or(0)
        .min(line_text.len());
    line_start_byte(buffer, line) + col
}

fn line_start_byte(buffer: &str, line: usize) -> usize {
    buffer
        .split('\n')
        .take(line)
        .map(|l| l.len() + 1)
        .sum::<usize>()
        .min(buffer.len())
}

fn displayed_text(
    value: Signal<String>,
    focused: Signal<bool>,
    placeholder: TextContent,
    preedit: Signal<String>,
    caret: Signal<usize>,
) -> Signal<String> {
    derived_eq(move || {
        let current = value.get();
        let composing = preedit.get();
        if !composing.is_empty() {
            splice_preedit(&current, caret.get(), &composing)
        } else if current.is_empty() && !focused.get() {
            text_content(&placeholder)
        } else {
            current
        }
    })
}

pub(super) fn splice_preedit(value: &str, caret: usize, preedit: &str) -> String {
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

fn text_content_untracked(content: &TextContent) -> String {
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
) -> sigil_style::StyleState {
    binary_style_state(interaction, disabled)
}

const fn edit_variant() -> StyleVariant {
    StyleVariant::new(
        EDIT_BASE,
        StateStyle::new(EDIT_REST, EDIT_FOCUS, EDIT_HOVER, EDIT_DOWN, EDIT_DISABLED),
    )
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
