use crate::app::Ambiguous;
use crate::style::Color;
use crate::terminal::glyphs::Glyphs;
use crate::terminal::graphics::{Mux, Protocol, detect, detect_mux};
use crate::terminal::term::{Calibration, ProbeRow};

macro_rules! fields {
    ($ty:ident { $($doc:literal $name:ident $set:ident: $field:ty,)* } { $($odoc:literal $oname:ident $oset:ident: $ofield:ty,)* }) => {
        impl $ty {
            $(
                #[doc = concat!("Returns ", $doc, ".")]
                #[must_use]
                pub const fn $name(&self) -> $field {
                    self.$name
                }

                #[cfg(feature = "kittest")]
                #[doc(hidden)]
                pub const fn $set(&mut self, $name: $field) {
                    self.$name = $name;
                }
            )*
            $(
                #[doc = concat!("Returns ", $odoc, ", if any.")]
                #[must_use]
                pub const fn $oname(&self) -> Option<$ofield> {
                    self.$oname
                }

                #[cfg(feature = "kittest")]
                #[doc(hidden)]
                pub fn $oset(&mut self, $oname: impl Into<Option<$ofield>>) {
                    self.$oname = $oname.into();
                }
            )*
        }
    };
}

#[cfg(unix)]
mod ask;
mod env;
use env::draft;
mod known;
#[cfg(unix)]
mod probe;
mod raw;

#[cfg(unix)]
use ask::{Question, Questions, Version};
pub(in crate::terminal) use known::{Known, known_env};
pub use known::{Notices, Notify};
#[cfg(unix)]
pub(crate) use probe::exchange;

/// The terminal had input waiting, so a question was not sent.
#[derive(Debug)]
pub(crate) struct Busy;

/// Asks the cell size in pixels; `Ok(None)` when the terminal does not answer.
#[cfg(unix)]
pub(crate) fn ask_cell() -> Result<Option<[u16; 2]>, Busy> {
    use std::io::Write;
    let input = probe::quiet_terminal().ok_or(Busy)?;
    let reply = probe::converse(
        &input,
        std::io::stdout().lock(),
        |out| out.write_all(b"\x1b[16t"),
        |text| probe::cell(text).is_some(),
    );
    Ok(reply.and_then(|reply| probe::cell(&reply)))
}

/// Asks the cell size in pixels; the answer is always empty off Unix.
#[cfg(not(unix))]
pub(crate) const fn ask_cell() -> Result<Option<[u16; 2]>, Busy> {
    Ok(None)
}

/// How many colors a terminal shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorDepth {
    /// The truecolor color depth.
    Truecolor,
    /// The ansi256 color depth.
    Ansi256,
    /// The ansi16 color depth.
    Ansi16,
    /// The monochrome color depth.
    Monochrome,
}

/// Terminal background brightness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backdrop {
    /// The dark backdrop.
    Dark,
    /// The light backdrop.
    Light,
}

/// The keyboard protocol a terminal speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Keyboard {
    /// The legacy keyboard.
    Legacy,
    /// The kitty keyboard.
    Kitty,
}

/// How the terminal host reaches the clipboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClipRoute {
    /// The pbcopy clip route.
    Pbcopy,
    /// The osc52 clip route.
    Osc52,
}

/// The characters the terminal host draws borders with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Borders {
    /// The rounded borders.
    Rounded,
    /// The light borders.
    Light,
    /// The ascii borders.
    Ascii,
}

/// How the terminal host fills a rectangle that covers part of a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fills {
    /// The partial fills.
    Partial,
    /// The whole fills.
    Whole,
}

/// How the terminal host shows the text cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cursor {
    /// The bar cursor.
    Bar,
    /// The block cursor.
    Block,
}

/// How clearing to the end of a line fills the cleared cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Erase {
    /// The background erase.
    Background,
    /// The default erase.
    Default,
}

const DEFAULT_CELL_ASPECT: f32 = 0.5;

/// Terminal capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub(crate) color: ColorDepth,
    pub(crate) graphics: Option<Protocol>,
    pub(crate) mux: Option<Mux>,
    pub(crate) keyboard: Keyboard,
    pub(crate) hyperlinks: bool,
    pub(crate) underline_styles: bool,
    pub(crate) borders: Borders,
    pub(crate) glyphs: Glyphs,
    pub(crate) ambiguous: Ambiguous,
    pub(crate) erase: Erase,
    pub(crate) cursor: Cursor,
    pub(crate) sync: bool,
    pub(crate) clipboard: ClipRoute,
    pub(crate) backdrop: Option<Backdrop>,
    pub(crate) background: Option<Color>,
    pub(crate) notices: Notices,
    pub(crate) fills: Fills,
    pub(crate) cell: Option<[u16; 2]>,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            color: ColorDepth::Truecolor,
            graphics: None,
            mux: None,
            keyboard: Keyboard::Kitty,
            hyperlinks: false,
            underline_styles: false,
            borders: Borders::Rounded,
            glyphs: Glyphs::Unicode,
            ambiguous: Ambiguous::Narrow,
            erase: Erase::Background,
            cursor: Cursor::Block,
            sync: true,
            clipboard: ClipRoute::Osc52,
            backdrop: None,
            background: None,
            notices: Notices::default(),
            fills: Fills::Partial,
            cell: None,
        }
    }
}

#[cfg(unix)]
#[derive(Debug, Default, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one independent yes or no per probe reply"
)]
struct Answers {
    sync: bool,
    kitty_keys: bool,
    backdrop: Option<Backdrop>,
    background: Option<Color>,
    truecolor: bool,
    version: Version,
    measured: Calibration,
    activity: bool,
    cell: Option<[u16; 2]>,
}

struct Draft {
    caps: Capabilities,
    #[cfg(unix)]
    ask: Questions,
}

fields!(Capabilities {
    "the color depth" color set_color: ColorDepth,
    "the keyboard protocol" keyboard set_keyboard: Keyboard,
    "whether the terminal shows hyperlinks" hyperlinks set_hyperlinks: bool,
    "whether the terminal draws underline styles" underline_styles set_underline_styles: bool,
    "the border characters" borders set_borders: Borders,
    "the glyph set" glyphs set_glyphs: Glyphs,
    "the width of ambiguous-width characters" ambiguous set_ambiguous: Ambiguous,
    "how clearing to the end of a line fills the cleared cells" erase set_erase: Erase,
    "the text cursor shape" cursor set_cursor: Cursor,
    "whether the terminal supports synchronized output" sync set_sync: bool,
    "how the host reaches the clipboard" clipboard set_clipboard: ClipRoute,
    "the startup notices" notices set_notices: Notices,
    "how the host fills a rectangle that covers part of a cell" fills set_fills: Fills,
} {
    "the image protocol" graphics set_graphics: Protocol,
    "the terminal multiplexer" mux set_mux: Mux,
    "the background brightness" backdrop set_backdrop: Backdrop,
    "the background color the terminal reported" background set_background: Color,
    "one cell's size in pixels as `[width, height]`" cell set_cell: [u16; 2],
});

impl Capabilities {
    /// One cell's width over its height, or 0.5 when the terminal did not report its cell size.
    #[must_use]
    pub fn cell_aspect(&self) -> f32 {
        self.cell.map_or(DEFAULT_CELL_ASPECT, |[width, height]| {
            f32::from(width) / f32::from(height)
        })
    }

    /// Detects capabilities from environment variables that `env` reads.
    #[must_use]
    pub fn detect(env: impl Fn(&str) -> Option<String>) -> Self {
        draft(&env).caps
    }

    /// Asks the terminal for its capabilities; uses the environment alone when raw mode fails.
    #[must_use]
    pub fn probe() -> Self {
        let env = |key: &str| std::env::var(key).ok();
        let Ok(_raw) = raw::Raw::enter() else {
            return Self::detect(env);
        };
        Self::measure(None).0
    }

    pub(in crate::terminal) fn measure(widths: Option<ProbeRow>) -> (Self, Calibration) {
        ask_terminal(draft(&|key: &str| std::env::var(key).ok()), widths)
    }
}

#[cfg(unix)]
fn ask_terminal(mut draft: Draft, widths: Option<ProbeRow>) -> (Capabilities, Calibration) {
    let backdrop = draft.caps.backdrop.is_none();
    draft.ask = draft.ask.with(Question::Keys, true);
    let Some(input) = probe::quiet_terminal() else {
        return (draft.caps, Calibration::Unmeasured);
    };
    match probe::ask_on(
        &input,
        std::io::stdout().lock(),
        draft.ask,
        (backdrop, widths),
    ) {
        Some(answers) if widths.is_some() => (draft.settle(&answers), answers.measured),
        Some(answers) => (draft.settle(&answers), Calibration::Unmeasured),
        None => (draft.caps, Calibration::Silent),
    }
}

#[cfg(not(unix))]
const fn ask_terminal(Draft { caps }: Draft, _: Option<ProbeRow>) -> (Capabilities, Calibration) {
    (caps, Calibration::Unmeasured)
}

#[cfg(unix)]
impl Draft {
    fn settle(mut self, answers: &Answers) -> Capabilities {
        self.caps.notices.activity = answers.activity;
        self.caps.cell = answers.cell.or(self.caps.cell);
        if self.ask.has(Question::Sync) {
            self.caps.sync = answers.sync;
        }
        if self.caps.backdrop.is_none() {
            self.caps.backdrop = answers.backdrop;
        }
        self.caps.background = answers.background.or(self.caps.background);
        if self.ask.has(Question::Keys) {
            self.caps.keyboard = if answers.kitty_keys {
                Keyboard::Kitty
            } else {
                Keyboard::Legacy
            };
        }
        if self.ask.has(Question::Color) && (answers.truecolor || answers.kitty_keys) {
            self.caps.color = ColorDepth::Truecolor;
        }
        if !self.ask.has(Question::Version) {
            return self.caps;
        }
        if !matches!(answers.version, Version::Silent) || answers.kitty_keys {
            self.caps.hyperlinks = true;
        }
        #[cfg(unix)]
        if let Version::Named(named) = answers.version {
            self.caps.notices = Notices {
                activity: answers.activity,
                ..named.notices()
            };
        }
        self.caps
    }
}

#[cfg(test)]
#[path = "caps_tests.rs"]
mod tests;

#[cfg(fuzz_replies)]
pub(crate) fn fuzz_reply(reply: &str) {
    std::hint::black_box(probe::parse(reply));
}
