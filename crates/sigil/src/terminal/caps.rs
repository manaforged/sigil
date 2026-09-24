use crate::terminal::graphics::{Mux, Protocol, detect, detect_mux};

#[cfg(unix)]
mod probe;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorDepth {
    Truecolor,
    Ansi256,
    Ansi16,
    Monochrome,
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
pub struct Caps {
    pub color: ColorDepth,
    pub graphics: Option<Protocol>,
    pub mux: Option<Mux>,
    pub keyboard: Keyboard,
    pub hyperlinks: bool,
    pub underline_styles: bool,
    pub borders: Borders,
    pub cursor: Cursor,
    pub sync: bool,
    pub clipboard: ClipRoute,
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
            cursor: Cursor::Block,
            sync: true,
            clipboard: ClipRoute::Osc52,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Known {
    Kitty,
    Ghostty,
    WezTerm,
    Iterm,
    Foot,
    Alacritty,
    Contour,
    WindowsTerminal,
    Vte,
    Apple,
}

#[derive(Default)]
struct Answers {
    graphics: Option<Protocol>,
    sync: bool,
    kitty_keys: bool,
}

#[derive(Clone, Copy)]
struct Questions {
    graphics: bool,
    sync: bool,
    keys: bool,
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
        let draft = draft(&|key: &str| std::env::var(key).ok());
        if !(draft.ask.graphics || draft.ask.sync || draft.ask.keys) {
            return draft.caps;
        }
        #[cfg(unix)]
        let answers = probe::ask(draft.ask);
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
        if self.ask.graphics {
            self.caps.graphics = answers.graphics;
        }
        if self.ask.sync {
            self.caps.sync = answers.sync;
        }
        if self.ask.keys && !answers.kitty_keys {
            self.caps.keyboard = Keyboard::Legacy;
        }
        self.caps
    }
}

fn draft(env: &impl Fn(&str) -> Option<String>) -> Draft {
    let known = known(env);
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
    Draft {
        caps: Caps {
            color: color_depth(env, known),
            graphics,
            mux: detect_mux(env),
            keyboard: if keys == Some(false) {
                Keyboard::Legacy
            } else {
                Keyboard::Kitty
            },
            hyperlinks: known.is_some_and(|term| term != Known::Apple),
            underline_styles: known.is_some_and(|term| {
                !matches!(
                    term,
                    Known::Apple | Known::Alacritty | Known::WindowsTerminal
                )
            }),
            borders: borders(env),
            cursor: if known.is_some_and(|term| term != Known::Apple) {
                Cursor::Bar
            } else {
                Cursor::Block
            },
            sync: sync.unwrap_or(true),
            clipboard: clip_route(env),
        },
        ask: Questions {
            graphics: graphics.is_none() && env("SIGIL_GRAPHICS").is_none(),
            sync: sync.is_none(),
            keys: keys.is_none(),
        },
    }
}

fn known(env: &impl Fn(&str) -> Option<String>) -> Option<Known> {
    let term = env("TERM").unwrap_or_default();
    let program = env("TERM_PROGRAM").unwrap_or_default();
    Some(
        if env("KITTY_WINDOW_ID").is_some() || term == "xterm-kitty" {
            Known::Kitty
        } else if env("GHOSTTY_RESOURCES_DIR").is_some() || program == "ghostty" {
            Known::Ghostty
        } else if env("WEZTERM_EXECUTABLE").is_some() || program == "WezTerm" {
            Known::WezTerm
        } else if program == "iTerm.app" || env("LC_TERMINAL").as_deref() == Some("iTerm2") {
            Known::Iterm
        } else if term.starts_with("foot") {
            Known::Foot
        } else if term == "alacritty" || env("ALACRITTY_WINDOW_ID").is_some() {
            Known::Alacritty
        } else if term.starts_with("contour") {
            Known::Contour
        } else if env("WT_SESSION").is_some() {
            Known::WindowsTerminal
        } else if env("VTE_VERSION").is_some() {
            Known::Vte
        } else if program == "Apple_Terminal" {
            Known::Apple
        } else {
            return None;
        },
    )
}

fn color_depth(env: &impl Fn(&str) -> Option<String>, known: Option<Known>) -> ColorDepth {
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

fn borders(env: &impl Fn(&str) -> Option<String>) -> Borders {
    let term = env("TERM").unwrap_or_default();
    let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
        .into_iter()
        .find_map(|key| env(key).filter(|value| !value.is_empty()))
        .map(|value| value.to_ascii_lowercase());
    let utf8 = locale.is_none_or(|value| value.contains("utf-8") || value.contains("utf8"));
    if term == "dumb" || !utf8 {
        Borders::Ascii
    } else if term == "linux" {
        Borders::Light
    } else {
        Borders::Rounded
    }
}

fn clip_route(env: &impl Fn(&str) -> Option<String>) -> ClipRoute {
    if cfg!(target_os = "macos") && env("SSH_TTY").is_none() {
        ClipRoute::Pbcopy
    } else {
        ClipRoute::Osc52
    }
}
