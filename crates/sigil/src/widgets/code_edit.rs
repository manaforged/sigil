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
    /// A line that was added; its bar uses the success color.
    Added,
    /// A line that was modified; its bar uses the info color.
    Modified,
    /// A line that was deleted; its bar uses the error color.
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
    /// A suggestion; the lowest severity, tinted like `Info`.
    Hint,
    /// A note for the reader, tinted with the info color.
    Info,
    /// A likely problem, tinted with the warning color.
    Warning,
    /// A definite problem; the highest severity, tinted with the error color.
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

/// A code diagnostic on a range of columns of one line.
#[derive(Clone, Copy, Debug)]
pub struct Diagnostic {
    /// The 0-based index of the line that carries the mark.
    pub(crate) line: usize,
    /// The first marked column of the line, 0-based.
    pub(crate) start: usize,
    /// The column after the last marked column.
    pub(crate) end: usize,
    /// The severity, which picks the tint of the mark.
    pub(crate) severity: Severity,
}

impl Diagnostic {
    /// Creates a diagnostic on `line` over the columns `start..end`.
    #[must_use]
    pub const fn new(line: usize, start: usize, end: usize, severity: Severity) -> Self {
        Self {
            line,
            start,
            end,
            severity,
        }
    }

    /// Returns the 0-based index of the line that carries the mark.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// Returns the first marked column of the line, 0-based.
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }

    /// Returns the column after the last marked column.
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Returns the severity, which picks the tint of the mark.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }
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
    /// Binds the selection anchor as `(line, column)`; the selection runs from it to the caret.
    pub const fn selection(mut self, anchor: Signal<Option<(usize, usize)>>) -> Self {
        self.selection = Some(anchor);
        self
    }

    /// Runs `f` for each key event while the editor is enabled, and keeps Tab in the editor.
    pub fn key_handler(mut self, f: impl Fn(&mut KeyEvent) + 'static) -> Self {
        self.on_key = Some(Rc::new(f));
        self
    }

    /// Disables the editor while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the syntax marks: one list of `Highlight` spans for each line.
    pub const fn highlights(mut self, spans: Signal<Vec<Vec<Highlight>>>) -> Self {
        self.highlights = Some(spans);
        self
    }

    /// Sets the height of each line row in px; the default is 18 px, or one cell on a cell grid.
    pub const fn row_height(mut self, h: f32) -> Self {
        self.row_h = Some(h);
        self
    }

    /// Shows a gutter with line numbers and an optional `Change` bar for each line.
    pub const fn gutter(mut self, marks: GutterMarks) -> Self {
        self.gutter = Some(marks);
        self
    }

    /// Sets the diagnostics; each one marks its range on its line by severity.
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
