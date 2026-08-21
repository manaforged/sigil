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

// Same chrome as `text_field`: surface fill + 1px border. The old
// "outer paints border/on-surface, inner is a fixed 358px pad" frame
// left a black slab whenever the author filled a wider box and focused.
static EDIT_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-inset"),
    StyleAtom::BorderWidth(Px(1)),
];
static EDIT_REST: &[StyleAtom] = &[StyleAtom::BorderToneStep(9)];
static EDIT_FOCUS: &[StyleAtom] = &[StyleAtom::BorderThemeRef("accent")];
static EDIT_HOVER: &[StyleAtom] = &[StyleAtom::BorderToneStep(10)];
static EDIT_DOWN: &[StyleAtom] = &[StyleAtom::BorderThemeRef("accent")];
static EDIT_DISABLED: &[StyleAtom] = &[StyleAtom::BorderToneStep(8), StyleAtom::Opacity(5_000)];

/// Cap on the number of physical line children we emit so a runaway
/// multi-megabyte paste doesn't blow up the tree on first paint.
///
/// Every line above this is not rendered, but the cap is now LOUD: a
/// trailing indicator line reports how many were withheld, so content
/// past the limit can never silently disappear (the old 64-line cap hid
/// a 65th line with no sign). A true fix — a scrolled viewport that only
/// renders visible lines — is the larger `text_edit` editor work, tracked
/// on the production-readiness plan.
const MAX_LINES: usize = 2000;

/// Default outer height in logical pixels for the text-edit chrome.
pub(super) const DEFAULT_HEIGHT: f32 = 160.0;
/// Border thickness in logical pixels — applied on every edge, so the
/// inner content area shrinks by `2 * BORDER_PX` along each axis.
const BORDER_PX: f32 = 1.0;
/// Horizontal padding between the outer (IME-anchor-carrying) node's
/// origin and the text content's left edge. `super::ime`'s caret-anchor
/// sync adds this to the raw text-engine geometry so the published
/// anchor matches where the glyphs actually paint (the reviewer adversarial
/// finding [W5-IME-TE] MEDIUM-3) — verified empirically: a laid-out
/// text child sits at `(12.0, 8.0)` relative to the outer node's
/// `(0, 0)` origin for this widget's fixed layout.
pub(super) const CONTENT_PADDING_X: f32 = 12.0;
/// Vertical counterpart of [`CONTENT_PADDING_X`].
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
    // Pointer handlers live HERE, on the content box, because it is the
    // deepest hittable node → the router binds pointer capture to it, so a
    // drag that leaves the widget still routes back (router.rs sets
    // `pointer_capture = path.last()`). Its children are non-hittable so
    // none of them becomes the capture target instead.
    let down = runtime.clone();
    let moved = runtime.clone();
    let scroll_y = scroll_into_view(runtime, frame_h);
    // The content wrapper carries BOTH the content padding and the scroll
    // offset in one absolute position, so the lines it flows and the
    // overlays it holds share one origin. `edit_inner` itself is unpadded
    // (padding lives here) — so the overlays inset from this origin without
    // re-adding padding, and the clip box is the full inner rect.
    let content_pos = derived(move || (CONTENT_PADDING_X, CONTENT_PADDING_Y - scroll_y.get()));
    let content = container()
        .hittable(false)
        .layout(LayoutStyle::block())
        .position_reactive(content_pos)
        // Order is paint order: selection behind the glyphs, caret in front.
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

/// Reactive vertical scroll offset that keeps the caret line in view.
///
/// An effect watches the caret + buffer and nudges the offset by the
/// minimum needed to bring the caret line inside `[scroll, scroll +
/// visible]`, so typing past the bottom scrolls down and arrowing back up
/// scrolls up, without jumping when the caret is already visible.
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
        #[allow(clippy::cast_precision_loss)]
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

/// Tinted-blue selection highlight — matches `text_field`.
static SELECTION_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundColor(
    sigil_style::ColorAtom::Rgba8(sigil_style::Rgba8 {
        r: 56,
        g: 118,
        b: 224,
        a: 130,
    }),
)];
/// Width shown for a selected trailing newline / empty line, so a
/// multi-line selection reads as continuous rather than a stack of gaps.
const NEWLINE_SEL_W: f32 = 6.0;

/// A selection highlight that spans lines: one rect per covered line,
/// partial on the first and last, full on the lines between. Driven by the
/// buffer's selection anchor (keyboard shift-select and, once wired,
/// drag-select).
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

/// Build one highlight rect per line covered by the byte range `lo..hi`.
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

/// One selection highlight rect for `line`, from `start_col` to `end_col`
/// (both clamped to the line). `None` when zero-width or geometry is
/// unavailable.
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
    // Lines before the last include the selected newline: show a small
    // trailing width so a fully-selected or empty line still reads.
    let min_w = if line < hi_line { NEWLINE_SEL_W } else { 0.0 };
    let width = (x1 - x0).max(min_w);
    if width <= 0.0 {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
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

/// Caret colour — the field's own text token.
static CARET_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("on-surface")];
/// Caret thickness in logical pixels.
const CARET_W: f32 = 1.5;
/// Line height fallback when no text-geometry provider is installed
/// (headless layout / golden tests without the engine).
const FALLBACK_LINE_H: f32 = 16.0;

/// An absolutely-positioned caret overlay that tracks the buffer's caret
/// across lines. Reactive on the caret offset, the buffer, focus and the
/// blink phase, so it appears only while focused and on-phase.
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

/// Build the caret rectangle for byte offset `byte` in `buffer`, or `None`
/// when geometry is unavailable. Insets are relative to the content
/// wrapper (which already carries the padding + scroll offset), so this is
/// just the caret's (line, column) position within the text.
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
    #[allow(clippy::cast_precision_loss)]
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

/// Line height from the installed text engine, or the headless fallback.
fn line_height() -> f32 {
    with_text_geometry(|engine| engine.caret("", &TextStyle::default(), 0).height)
        .filter(|h| *h > 0.0)
        .unwrap_or(FALLBACK_LINE_H)
}

/// Byte `caret` in `buffer` → (line index, byte offset within that line).
fn caret_line_col(buffer: &str, caret: usize) -> (usize, usize) {
    let caret = caret.min(buffer.len());
    let before = &buffer[..caret];
    let line = before.bytes().filter(|&b| b == b'\n').count();
    let col = before.rfind('\n').map_or(caret, |nl| caret - (nl + 1));
    (line, col)
}

/// The `n`th `\n`-delimited line of `buffer` (empty when out of range).
fn nth_line(buffer: &str, n: usize) -> &str {
    buffer.split('\n').nth(n).unwrap_or("")
}

fn edit_inner_layout() -> LayoutStyle {
    // Fills the frame. A fixed inner size left a strip of frame chrome
    // (on-surface when focused) beside the text.
    LayoutStyle::flex_column()
        .fill()
        .min_size(0.0, 0.0)
        .overflow_y(Overflow::Hidden)
}

/// Render the buffer as one child text node per `\n`-delimited line.
///
/// Each line is a `text(line)` inside a `flex_column`. The line set is
/// installed via `children_reactive`: when the bound `displayed` signal
/// changes line count (e.g. typing `Enter`), the parent's child list
/// rebuilds against the fresh line vector. The text engine handles
/// intra-line shaping and (where supported) wrapping; line-aware caret
/// math is still followup work — see the module docs.
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
        // Never hide content silently: report the withheld lines so the
        // limit is visible instead of a phantom truncation.
        nodes.push(
            text(notice)
                .layout(LayoutStyle::block().fill_width().min_size(0.0, 0.0))
                .class("text-muted-foreground"),
        );
    }
    nodes
}

/// The withheld-lines notice for a `total`-line document, or `None` when
/// nothing is hidden. Pure so the truncation contract is testable without
/// laying out thousands of line nodes.
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

/// Drag-start: place the caret at the press and capture so the drag
/// routes back here even if it leaves the widget. `capture_pointer` binds
/// to the HIT TARGET, so the content below must be non-hittable or it
/// would steal the capture (router.rs binds `pointer_capture` to
/// `path.last()`).
fn pointer_drag_start(event: &mut PointerEvent, runtime: &TextEditRuntime, scroll_y: Signal<f32>) {
    let buffer = runtime.value.peek();
    let offset = offset_at_point(&buffer, event.position.x, event.position.y, scroll_y.peek());
    let mut edit = runtime.edit_mut();
    edit.place_caret(offset);
    super::handlers::commit(runtime, &edit);
    event.capture_pointer();
}

/// Drag-move: extend the selection to the current position.
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

/// Pointer position (node-local) → nearest buffer byte offset. Inverse of
/// the caret geometry: `y` selects the line, `x` the column within it via
/// the text engine's hit-test.
fn offset_at_point(buffer: &str, x: f32, y: f32, scroll_y: f32) -> usize {
    let line_h = line_height();
    // The content is shifted up by `scroll_y`, so a pointer at screen `y`
    // maps to a document position `scroll_y` further down.
    let content_y = (y - CONTENT_PADDING_Y + scroll_y).max(0.0);
    let content_x = (x - CONTENT_PADDING_X).max(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let raw_line = (content_y / line_h) as usize;
    let last_line = buffer.bytes().filter(|&b| b == b'\n').count();
    let line = raw_line.min(last_line);
    let line_text = nth_line(buffer, line);
    let col = with_text_geometry(|engine| engine.hit(line_text, &TextStyle::default(), content_x))
        .unwrap_or(0)
        .min(line_text.len());
    line_start_byte(buffer, line) + col
}

/// Byte offset where `line` starts (sum of earlier lines' bytes plus their
/// newline terminators). Clamped to the buffer length.
fn line_start_byte(buffer: &str, line: usize) -> usize {
    buffer
        .split('\n')
        .take(line)
        .map(|l| l.len() + 1)
        .sum::<usize>()
        .min(buffer.len())
}

/// The editor's displayed text: the committed buffer with any in-flight
/// IME preedit spliced in at the caret for display only (`value` never
/// carries preedit text — mirrors `text_field::render::displayed_text`).
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

/// Insert `preedit` into `value` at the `caret` byte offset, for display
/// only. `caret` normally mirrors `TextEdit`'s caret (always a valid
/// UTF-8 boundary by construction), but the mirror is published by a
/// separate signal write than `value` — a reader can observe a
/// transient `(value, caret)` pair where `caret` was valid for the
/// *previous* `value` but not the current one (e.g. a multibyte
/// backspace shrinks `value` out from under a stale `caret` before
/// `commit` catches up). `batch`ing both writes at the call site
/// closes that window (the reviewer adversarial finding [W5-IME-TE] HIGH-1),
/// but this defensively clamps to the nearest real boundary too, so a
/// caller that ever reads a genuinely inconsistent pair degrades to a
/// slightly-off splice point instead of panicking.
pub(super) fn splice_preedit(value: &str, caret: usize, preedit: &str) -> String {
    let caret = floor_char_boundary(value, caret);
    let mut out = String::with_capacity(value.len() + preedit.len());
    out.push_str(&value[..caret]);
    out.push_str(preedit);
    out.push_str(&value[caret..]);
    out
}

/// Clamp `byte` to the nearest UTF-8 boundary at or below `text.len()`.
/// Stable equivalent of the still-nightly `str::floor_char_boundary`.
/// Shared by `splice_preedit` and `super::ime`'s preedit-cursor clamp.
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
