use crate::app::Ambiguous;
use crate::terminal::glyphs::Glyphs;
use crate::terminal::graphics::{Mux, Protocol, detect, detect_mux, forced_graphics};

mod ask;
mod known;
#[cfg(unix)]
mod probe;

use ask::{Question, Questions, Version};
pub(in crate::terminal) use known::{Known, known_env};
pub use known::{Notices, Notify};
#[cfg(unix)]
pub(crate) use probe::exchange;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorDepth {
    Truecolor,
    Ansi256,
    Ansi16,
    Monochrome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backdrop {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyboard {
    Legacy,
    Kitty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipRoute {
    Pbcopy,
    Osc52,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Borders {
    Rounded,
    Light,
    Ascii,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Bar,
    Block,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Erase {
    Background,
    Default,
}

fn erase_for(term: Option<&str>) -> Erase {
    if term.is_some_and(|term| term.starts_with("screen")) {
        Erase::Default
    } else {
        Erase::Background
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caps {
    pub color: ColorDepth,
    pub graphics: Option<Protocol>,
    pub mux: Option<Mux>,
    pub keyboard: Keyboard,
    pub hyperlinks: bool,
    pub underline_styles: bool,
    pub borders: Borders,
    pub glyphs: Glyphs,
    pub ambiguous: Ambiguous,
    pub erase: Erase,
    pub cursor: Cursor,
    pub sync: bool,
    pub clipboard: ClipRoute,
    pub backdrop: Option<Backdrop>,
    pub notices: Notices,
}

impl Default for Caps {
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
            notices: Notices {
                notify: None,
                progress: false,
            },
        }
    }
}

#[derive(Default)]
struct Answers {
    graphics: Option<Protocol>,
    sync: bool,
    kitty_keys: bool,
    backdrop: Option<Backdrop>,
    truecolor: bool,
    version: Version,
}

struct Draft {
    caps: Caps,
    ask: Questions,
}

impl Caps {
    #[must_use]
    pub const fn host(&self) -> crate::app::HostCaps {
        crate::app::HostCaps {
            hyperlinks: self.hyperlinks,
        }
    }

    #[must_use]
    pub fn detect(env: impl Fn(&str) -> Option<String>) -> Self {
        draft(&env).caps
    }

    #[must_use]
    pub fn probe() -> Self {
        let mut draft = draft(&|key: &str| std::env::var(key).ok());
        #[cfg(unix)]
        {
            draft.caps.keyboard = keyboard(probe::keys());
        }
        let backdrop = draft.caps.backdrop.is_none();
        if draft.ask.none() && !backdrop {
            return draft.caps;
        }
        #[cfg(unix)]
        let answers = probe::ask(draft.ask, backdrop);
        #[cfg(not(unix))]
        let answers: Option<Answers> = None;
        match answers {
            Some(answers) => draft.settle(&answers),
            None => draft.caps,
        }
    }
}

impl Draft {
    const fn settle(mut self, answers: &Answers) -> Caps {
        if self.ask.has(Question::Graphics) {
            self.caps.graphics = answers.graphics;
        }
        if self.ask.has(Question::Sync) {
            self.caps.sync = answers.sync;
        }
        if self.caps.backdrop.is_none() {
            self.caps.backdrop = answers.backdrop;
        }
        if self.ask.has(Question::Keys) && answers.kitty_keys {
            self.caps.keyboard = Keyboard::Kitty;
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
            self.caps.notices = named.notices();
        }
        self.caps
    }
}

fn draft(env: &impl Fn(&str) -> Option<String>) -> Draft {
    let mux = detect_mux(env);
    let known = known::known(env, mux);
    let sync = known.map(|term| !matches!(term, Known::Vte | Known::Apple));
    let keys = known.map(|term| {
        matches!(
            term,
            Known::Kitty
                | Known::Ghostty
                | Known::WezTerm
                | Known::Iterm
                | Known::Foot
                | Known::Alacritty
        )
    });
    let graphics = detect(env);
    let glyphs = glyphs(env);
    let color = color_depth(env, known);
    Draft {
        caps: Caps {
            color,
            graphics,
            mux,
            keyboard: keyboard(keys),
            hyperlinks: known.is_some_and(|term| term != Known::Apple),
            underline_styles: known.is_some_and(|term| {
                !matches!(
                    term,
                    Known::Apple | Known::Alacritty | Known::WindowsTerminal
                )
            }),
            borders: borders(glyphs),
            glyphs,
            ambiguous: ambiguous(env),
            erase: erase_for(env("TERM").as_deref()),
            cursor: if known.is_some_and(|term| term != Known::Apple) {
                Cursor::Bar
            } else {
                Cursor::Block
            },
            sync: sync.unwrap_or(true),
            clipboard: clip_route(env),
            backdrop: backdrop(env),
            notices: known.map(Known::notices).unwrap_or_default(),
        },
        ask: questions(env, (graphics, sync, keys), color, known),
    }
}

fn questions(
    env: &impl Fn(&str) -> Option<String>,
    (graphics, sync, keys): (Option<Protocol>, Option<bool>, Option<bool>),
    color: ColorDepth,
    known: Option<Known>,
) -> Questions {
    Questions::default()
        .with(
            Question::Graphics,
            graphics.is_none() && forced_graphics(env).is_none(),
        )
        .with(Question::Sync, sync.is_none())
        .with(Question::Keys, keys.is_none())
        .with(
            Question::Color,
            matches!(color, ColorDepth::Ansi256 | ColorDepth::Ansi16)
                && forced_color(env).is_none(),
        )
        .with(Question::Version, known.is_none())
}

fn forced_color(env: &impl Fn(&str) -> Option<String>) -> Option<ColorDepth> {
    color_override(&env("SIGIL_COLOR")?)
}

fn color_override(value: &str) -> Option<ColorDepth> {
    match value.trim().to_ascii_lowercase().as_str() {
        "truecolor" | "24bit" => Some(ColorDepth::Truecolor),
        "256" => Some(ColorDepth::Ansi256),
        "16" => Some(ColorDepth::Ansi16),
        "none" => Some(ColorDepth::Monochrome),
        _ => None,
    }
}

fn flag(env: &impl Fn(&str) -> Option<String>, key: &str) -> bool {
    env(key).is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn color_depth(env: &impl Fn(&str) -> Option<String>, known: Option<Known>) -> ColorDepth {
    if let Some(forced) = forced_color(env) {
        return forced;
    }
    if env("NO_COLOR").is_some_and(|value| !value.is_empty()) {
        return ColorDepth::Monochrome;
    }
    let term = env("TERM").unwrap_or_default();
    let colorterm = env("COLORTERM").unwrap_or_default().to_ascii_lowercase();
    if term == "dumb" {
        ColorDepth::Monochrome
    } else if colorterm.contains("truecolor")
        || colorterm.contains("24bit")
        || term.ends_with("-direct")
        || known.is_some_and(|found| found != Known::Apple)
    {
        ColorDepth::Truecolor
    } else if term.contains("256") || known == Some(Known::Apple) {
        ColorDepth::Ansi256
    } else if !term.is_empty() {
        ColorDepth::Ansi16
    } else if cfg!(windows) {
        ColorDepth::Truecolor
    } else {
        ColorDepth::Monochrome
    }
}

fn glyphs(env: &impl Fn(&str) -> Option<String>) -> Glyphs {
    if flag(env, "SIGIL_ASCII") {
        return Glyphs::Ascii;
    }
    let term = env("TERM").unwrap_or_default();
    let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
        .into_iter()
        .find_map(|key| env(key).filter(|value| !value.is_empty()))
        .map(|value| value.to_ascii_lowercase());
    let utf8 = locale.is_none_or(|value| value.contains("utf-8") || value.contains("utf8"));
    if !utf8 || matches!(term.as_str(), "dumb" | "vt100" | "vt102" | "vt220") {
        Glyphs::Ascii
    } else if term == "linux" {
        Glyphs::Console
    } else {
        Glyphs::Unicode
    }
}

const fn keyboard(known: Option<bool>) -> Keyboard {
    if matches!(known, Some(true)) {
        Keyboard::Kitty
    } else {
        Keyboard::Legacy
    }
}

fn ambiguous(env: &impl Fn(&str) -> Option<String>) -> Ambiguous {
    if flag(env, "SIGIL_AMBIGUOUS_WIDE") {
        Ambiguous::Wide
    } else {
        Ambiguous::Narrow
    }
}

const fn borders(glyphs: Glyphs) -> Borders {
    match glyphs {
        Glyphs::Ascii => Borders::Ascii,
        Glyphs::Console => Borders::Light,
        Glyphs::Unicode => Borders::Rounded,
    }
}

fn clip_route(env: &impl Fn(&str) -> Option<String>) -> ClipRoute {
    if cfg!(target_os = "macos") && env("SSH_TTY").is_none() {
        ClipRoute::Pbcopy
    } else {
        ClipRoute::Osc52
    }
}

fn backdrop(env: &impl Fn(&str) -> Option<String>) -> Option<Backdrop> {
    let pair = env("COLORFGBG")?;
    let index: u8 = pair.rsplit(';').next()?.parse().ok()?;
    Some(if matches!(index, 7 | 15) {
        Backdrop::Light
    } else {
        Backdrop::Dark
    })
}

#[cfg(test)]
#[path = "caps_tests.rs"]
mod tests;
