mod render;
mod scroll;

use std::rc::Rc;

use crate::events::prelude::{KeyEvent, NodeEventsExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, container};
use crate::layout::LayoutReactiveExt;
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle, Overflow};
use crate::reactive::{Signal, derived_eq, effect, signal};
use crate::style::NodeStyleExt;

use scroll::{first_row, follow_caret, scroll_body, wheel};

const ROW_H: f32 = 18.0;
const CARET_W: f32 = 2.0;
const CHANGE_BAR_W: f32 = 2.0;
const DIGIT_W: f32 = 7.0;
const GUTTER_PAD: f32 = 8.0;
const SELECTION_CLASS: &str = "bg-accent";
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
    const fn bar_class(self) -> &'static str {
        match self {
            Self::Added => "bg-success-9",
            Self::Modified => "bg-info-9",
            Self::Deleted => "bg-error-9",
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
    const fn tint_class(self) -> &'static str {
        match self {
            Self::Error => "bg-error-3",
            Self::Warning => "bg-warn-3",
            Self::Info | Self::Hint => "bg-info-3",
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
    row_h: f32,
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
        row_h: ROW_H,
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
    pub const fn highlights(mut self, spans: Signal<Vec<Vec<HlSpan>>>) -> Self {
        self.highlights = Some(spans);
        self
    }

    #[must_use]
    pub const fn row_height(mut self, h: f32) -> Self {
        self.row_h = h;
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
        let lines = self.lines;
        let caret = self.caret;
        let highlights = self.highlights;
        let selection = self.selection;
        let gutter = self.gutter;
        let diagnostics = self.diagnostics;
        let row_h = self.row_h;
        let traps_tab = self.on_key.is_some();
        let on_key = self.on_key.unwrap_or_else(|| Rc::new(|_| {}));
        let scroll_y = signal(0.0_f32);
        let viewport = signal((0.0_f32, DEFAULT_VIEWPORT_H));
        let start = derived_eq(move || first_row(scroll_y.get(), lines.get().len(), row_h));
        effect(move || follow_caret(scroll_y, caret, lines, viewport, row_h));
        container()
            .role(Role::TextInput)
            .label("Code editor")
            .focusable(true)
            .traps_tab(traps_tab)
            .class("w-full h-full")
            .layout(
                LayoutStyle::flex_column()
                    .overflow_x(Overflow::Hidden)
                    .overflow_y(Overflow::Hidden),
            )
            .size_reactive(viewport)
            .on_scroll(move |ev| wheel(ev, scroll_y, lines, viewport, row_h))
            .key_any(move |event| on_key(event))
            .child(scroll_body(
                Surface {
                    lines,
                    caret,
                    highlights,
                    selection,
                    gutter,
                    diagnostics,
                    start,
                    viewport,
                },
                row_h,
            ))
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
