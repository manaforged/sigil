mod a11y;
mod editing;
mod render;
mod scroll;

use super::highlight::Highlight;
use crate::layout::Space;
use crate::reactive::Reactive;
use crate::style::locale::Direction;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use std::rc::Rc;

use crate::events::KeyEvent;
use crate::kernel::{NodeBuilder, Role, container};
use crate::widgets::{Layout, Widget};

use crate::layout::snap::{CellForm, line};
use crate::layout::{LayoutStyle, Overflow};
use crate::reactive::{Signal, effect, memo, signal};
use crate::style::{StyleOverlay, Token, space};

use super::shared::{FIELD_VARIANT, disabled_a11y, disabled_now, reactive_style_state};
use crate::widgets::interaction::InteractionState;
use scroll::{first_row, follow_caret, scroll_body, wheel};

const CHANGE_BAR_W: f32 = 2.0;
const GUTTER_PAD: f32 = space(2);
const DIGIT_W: f32 = 7.0;
const SELECTION: Token = Token::Accent;
const RUN_CLASS: &str = "text-sm font-mono";
const ROW_BUFFER: usize = 6;
const DEFAULT_VIEWPORT_H: f32 = 1600.0;

type KeyHandler = Rc<dyn Fn(&mut KeyEvent) + 'static>;

type SelectionAnchor = Signal<Option<(usize, usize)>>;
type SelRange = ((usize, usize), (usize, usize));

type GutterMarks = Signal<Vec<Option<Change>>>;

/// Line change mark in the `code_edit` gutter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Change {
    /// The added change kind.
    Added,
    /// The modified change kind.
    Modified,
    /// The deleted change kind.
    Deleted,
}

impl Change {
    const fn bar_token(self) -> Token {
        match self {
            Self::Added => Token::Success9,
            Self::Modified => Token::Info9,
            Self::Deleted => Token::Error9,
        }
    }
}

/// The severity of a code diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The hint diag severity.
    Hint,
    /// The info diag severity.
    Info,
    /// The warning diag severity.
    Warning,
    /// The error diag severity.
    Error,
}

impl Severity {
    const fn tint_token(self) -> Token {
        match self {
            Self::Error => Token::Error3,
            Self::Warning => Token::Warning3,
            Self::Info | Self::Hint => Token::Info3,
        }
    }
}

/// A code diagnostic on a byte range.
#[derive(Clone, Copy, Debug)]
pub struct Diagnostic {
    /// The line.
    pub line: usize,
    /// The start.
    pub start: usize,
    /// The end.
    pub end: usize,
    /// The severity.
    pub severity: Severity,
}

type Diagnostics = Signal<Vec<Diagnostic>>;

/// Builder of a code editor.
#[must_use]
pub struct CodeEditBuilder {
    lines: Signal<Vec<String>>,
    caret: Signal<(usize, usize)>,
    highlights: Option<Signal<Vec<Vec<Highlight>>>>,
    selection: Option<SelectionAnchor>,
    gutter: Option<GutterMarks>,
    diagnostics: Option<Diagnostics>,
    on_key: Option<KeyHandler>,
    disabled: Option<Signal<bool>>,
    row_h: Option<f32>,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for CodeEditBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CodeEditBuilder")
            .field("lines", &self.lines)
            .field("caret", &self.caret)
            .field("highlights", &self.highlights)
            .field("selection", &self.selection)
            .field("gutter", &self.gutter)
            .field("diagnostics", &self.diagnostics)
            .field("disabled", &self.disabled)
            .field("row_h", &self.row_h)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A code editor with built-in `text_edit` editing.
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
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl CodeEditBuilder {
    /// Sets the selection.
    pub const fn selection(mut self, anchor: Signal<Option<(usize, usize)>>) -> Self {
        self.selection = Some(anchor);
        self
    }

    /// Sets the key handler.
    pub fn key_handler(mut self, f: impl Fn(&mut KeyEvent) + 'static) -> Self {
        self.on_key = Some(Rc::new(f));
        self
    }

    /// Sets the disabled.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the highlights.
    pub const fn highlights(mut self, spans: Signal<Vec<Vec<Highlight>>>) -> Self {
        self.highlights = Some(spans);
        self
    }

    /// Sets the row height.
    pub const fn row_height(mut self, h: f32) -> Self {
        self.row_h = Some(h);
        self
    }

    /// Sets the gutter.
    pub const fn gutter(mut self, marks: GutterMarks) -> Self {
        self.gutter = Some(marks);
        self
    }

    /// Sets the diagnostics.
    pub const fn diagnostics(mut self, spans: Diagnostics) -> Self {
        self.diagnostics = Some(spans);
        self
    }

    fn root(self) -> NodeBuilder {
        let model = editing::EditModel {
            lines: self.lines,
            caret: self.caret,
            selection: self.selection,
        };
        let lines = self.lines;
        let row_h = self.row_h.unwrap_or_else(|| line(18.0));
        let traps_tab = self.on_key.is_some();
        let on_key = self.on_key.unwrap_or_else(|| Rc::new(|_| {}));
        let scroll_y = signal(0.0_f32);
        let viewport = signal((0.0_f32, DEFAULT_VIEWPORT_H));
        let interaction = InteractionState::new(true);
        let disabled = self.disabled;
        let start = memo(move || first_row(scroll_y.get(), lines.get().len(), row_h));
        effect(move || follow_caret(scroll_y, self.caret, lines, viewport, row_h));
        let runtime = editing::runtime(model, self.disabled);
        let node = field_chrome(container(), interaction, disabled)
            .role(Role::TextInput)
            .label(theme_label(StringKey::CodeEditor))
            .with(Direction::Ltr)
            .focusable(true)
            .traps_tab(traps_tab)
            .class("w-full h-full")
            .layout(
                LayoutStyle::flex_column()
                    .padding(Space::px(CellForm::new(space(1), 0.0).pick()))
                    .overflow_x(Overflow::Hidden)
                    .overflow_y(Overflow::Hidden),
            )
            .size_reactive(viewport)
            .scroll(move |ev| wheel(ev, scroll_y, lines, viewport, row_h))
            .key_capture_any(gated(on_key, self.disabled))
            .child(scroll_body(
                EditSurface {
                    lines,
                    caret: self.caret,
                    highlights: self.highlights,
                    selection: self.selection,
                    gutter: self.gutter,
                    diagnostics: self.diagnostics,
                    start,
                    viewport,
                    caret_shown: caret_shown(interaction.focused()),
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
    .seal_style()
    .focus_ring(interaction.focus_ring())
}

fn caret_shown(focused: Signal<bool>) -> Signal<bool> {
    let always = CellForm::new(true, false).pick();
    memo(move || always || focused.get())
}

fn gated(on_key: KeyHandler, disabled: Option<Signal<bool>>) -> impl Fn(&mut KeyEvent) + 'static {
    move |event| {
        if !disabled_now(disabled) {
            on_key(event);
        }
    }
}

#[derive(Clone, Copy)]
struct EditSurface {
    lines: Signal<Vec<String>>,
    caret: Signal<(usize, usize)>,
    highlights: Option<Signal<Vec<Vec<Highlight>>>>,
    selection: Option<SelectionAnchor>,
    gutter: Option<GutterMarks>,
    diagnostics: Option<Diagnostics>,
    start: Signal<usize>,
    viewport: Signal<(f32, f32)>,
    caret_shown: Signal<bool>,
}

#[derive(Clone, Copy)]
struct LineWindow {
    start: usize,
    budget: usize,
    row_h: f32,
}

builder_impls!([] CodeEditBuilder: widget, node);
