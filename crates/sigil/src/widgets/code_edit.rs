mod a11y;
mod editing;
mod render;
mod scroll;

use std::rc::Rc;

use crate::events::prelude::{KeyEvent, NodeEventsExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, container};
use crate::layout::LayoutReactiveExt;
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle, Overflow};
use crate::layout::snap::{CellForm, line};
use crate::reactive::{Signal, derived, effect, signal};
use crate::style::{NodeStyleExt, StyleOverlay, Token, space};

use super::shared::{FIELD_VARIANT, disabled_a11y, disabled_now, reactive_style_state};
use crate::widgets::interaction::InteractionState;
use scroll::{first_row, follow_caret, scroll_body, wheel};

const ROW_H: f32 = 18.0;
const CARET_W: f32 = 2.0;
const CHANGE_BAR_W: f32 = 2.0;
const DIGIT_W: f32 = 7.0;
const GUTTER_PAD: f32 = space(2);
const SELECTION: Token = Token::Accent;
const RUN_CLASS: &str = "text-sm font-mono";
const ROW_BUFFER: usize = 6;
const DEFAULT_VIEWPORT_H: f32 = 1600.0;

type KeyHandler = Rc<dyn Fn(&mut KeyEvent) + 'static>;

type SelectionAnchor = Signal<Option<(usize, usize)>>;
type SelRange = ((usize, usize), (usize, usize));

type GutterMarks = Signal<Vec<Option<ChangeKind>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
}

impl ChangeKind {
    const fn bar_token(self) -> Token {
        match self {
            Self::Added => Token::Success9,
            Self::Modified => Token::Info9,
            Self::Deleted => Token::Error9,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HlSpan {
    pub start: usize,
    pub end: usize,
    pub token: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagSeverity {
    Hint,
    Info,
    Warning,
    Error,
}

impl DiagSeverity {
    const fn tint_token(self) -> Token {
        match self {
            Self::Error => Token::Error3,
            Self::Warning => Token::Warn3,
            Self::Info | Self::Hint => Token::Info3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DiagSpan {
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub severity: DiagSeverity,
}

type DiagSpans = Signal<Vec<DiagSpan>>;

pub struct CodeEditBuilder {
    lines: Signal<Vec<String>>,
    caret: Signal<(usize, usize)>,
    highlights: Option<Signal<Vec<Vec<HlSpan>>>>,
    selection: Option<SelectionAnchor>,
    gutter: Option<GutterMarks>,
    diagnostics: Option<DiagSpans>,
    on_key: Option<KeyHandler>,
    disabled: Option<Signal<bool>>,
    row_h: Option<f32>,
}

#[must_use]
pub fn code_edit(lines: Signal<Vec<String>>, caret: Signal<(usize, usize)>) -> CodeEditBuilder {
    CodeEditBuilder {
        lines,
        caret,
        highlights: None,
        selection: None,
        gutter: None,
        diagnostics: None,
        on_key: None,
        disabled: None,
        row_h: None,
    }
}

impl CodeEditBuilder {
    #[must_use]
    pub const fn selection(mut self, anchor: Signal<Option<(usize, usize)>>) -> Self {
        self.selection = Some(anchor);
        self
    }

    #[must_use]
    pub fn key(mut self, f: impl Fn(&mut KeyEvent) + 'static) -> Self {
        self.on_key = Some(Rc::new(f));
        self
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    #[must_use]
    pub const fn highlights(mut self, spans: Signal<Vec<Vec<HlSpan>>>) -> Self {
        self.highlights = Some(spans);
        self
    }

    #[must_use]
    pub const fn row_height(mut self, h: f32) -> Self {
        self.row_h = Some(h);
        self
    }

    #[must_use]
    pub const fn gutter(mut self, marks: GutterMarks) -> Self {
        self.gutter = Some(marks);
        self
    }

    #[must_use]
    pub const fn diagnostics(mut self, spans: DiagSpans) -> Self {
        self.diagnostics = Some(spans);
        self
    }

    fn into_node(self) -> NodeBuilder {
        let model = editing::Model {
            lines: self.lines,
            caret: self.caret,
            selection: self.selection,
        };
        let lines = self.lines;
        let row_h = self.row_h.unwrap_or_else(|| line(ROW_H));
        let traps_tab = self.on_key.is_some();
        let on_key = self.on_key.unwrap_or_else(|| Rc::new(|_| {}));
        let scroll_y = signal(0.0_f32);
        let viewport = signal((0.0_f32, DEFAULT_VIEWPORT_H));
        let interaction = InteractionState::new(true);
        let disabled = self.disabled;
        let start = derived(move || first_row(scroll_y.get(), lines.get().len(), row_h));
        effect(move || follow_caret(scroll_y, self.caret, lines, viewport, row_h));
        let runtime = editing::runtime(model, self.disabled);
        let node = field_chrome(container(), interaction, disabled)
            .role(Role::TextInput)
            .label("Code editor")
            .focusable(true)
            .traps_tab(traps_tab)
            .class("w-full h-full")
            .layout(
                LayoutStyle::flex_column()
                    .padding(CellForm::new(space(1), 0.0).pick())
                    .overflow_x(Overflow::Hidden)
                    .overflow_y(Overflow::Hidden),
            )
            .size_reactive(viewport)
            .on_scroll(move |ev| wheel(ev, scroll_y, lines, viewport, row_h))
            .key_capture_any(gated(on_key, self.disabled))
            .child(scroll_body(
                Surface {
                    lines,
                    caret: self.caret,
                    highlights: self.highlights,
                    selection: self.selection,
                    gutter: self.gutter,
                    diagnostics: self.diagnostics,
                    start,
                    viewport,
                    caret_shown: caret_shown(interaction.focus_visible()),
                },
                row_h,
            ));
        let node = editing::attach(a11y::expose(node, &runtime), &runtime, viewport, row_h);
        interaction.observe_style(disabled_a11y(node, disabled))
    }
}

fn field_chrome(
    node: NodeBuilder,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> NodeBuilder {
    node.stateful_style(FIELD_VARIANT, StyleOverlay::default(), move || {
        reactive_style_state(interaction, disabled)
    })
    .focus_ring(interaction.focus_ring())
}

fn caret_shown(focused: Signal<bool>) -> Signal<bool> {
    let always = CellForm::new(true, false).pick();
    derived(move || always || focused.get())
}

fn gated(on_key: KeyHandler, disabled: Option<Signal<bool>>) -> impl Fn(&mut KeyEvent) + 'static {
    move |event| {
        if !disabled_now(disabled) {
            on_key(event);
        }
    }
}

#[derive(Clone, Copy)]
struct Surface {
    lines: Signal<Vec<String>>,
    caret: Signal<(usize, usize)>,
    highlights: Option<Signal<Vec<Vec<HlSpan>>>>,
    selection: Option<SelectionAnchor>,
    gutter: Option<GutterMarks>,
    diagnostics: Option<DiagSpans>,
    start: Signal<usize>,
    viewport: Signal<(f32, f32)>,
    caret_shown: Signal<bool>,
}

#[derive(Clone, Copy)]
struct Window {
    start: usize,
    budget: usize,
    row_h: f32,
}

impl IntoNode for CodeEditBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node().build(cx)
    }
}
