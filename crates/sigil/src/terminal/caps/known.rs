use crate::terminal::graphics::Mux;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notify {
    Osc9,
    Osc777,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Notices {
    pub notify: Option<Notify>,
    pub progress: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum Known {
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

#[derive(Clone, Copy)]
enum Name {
    Is(&'static str),
    Starts(&'static str),
}

#[derive(Clone, Copy)]
enum Signal {
    Set(&'static str),
    Is(&'static str, &'static str),
}

struct Terminal {
    known: Known,
    term: Option<Name>,
    env: &'static [Signal],
    version: Option<&'static str>,
    kitty_keys: bool,
    underline_styles: bool,
    sync: bool,
    notices: Notices,
}

const fn notices(notify: Option<Notify>, progress: bool) -> Notices {
    Notices { notify, progress }
}

const TERMINALS: [Terminal; 10] = [
    Terminal {
        known: Known::Kitty,
        term: Some(Name::Is("xterm-kitty")),
        env: &[Signal::Set("KITTY_WINDOW_ID")],
        version: Some("kitty"),
        kitty_keys: true,
        underline_styles: true,
        sync: true,
        notices: notices(Some(Notify::Osc9), true),
    },
    Terminal {
        known: Known::Ghostty,
        term: Some(Name::Is("xterm-ghostty")),
        env: &[
            Signal::Set("GHOSTTY_RESOURCES_DIR"),
            Signal::Is("TERM_PROGRAM", "ghostty"),
        ],
        version: Some("ghostty"),
        kitty_keys: true,
        underline_styles: true,
        sync: true,
        notices: notices(Some(Notify::Osc777), true),
    },
    Terminal {
        known: Known::WezTerm,
        term: Some(Name::Is("wezterm")),
        env: &[
            Signal::Set("WEZTERM_EXECUTABLE"),
            Signal::Is("TERM_PROGRAM", "WezTerm"),
        ],
        version: Some("wezterm"),
        kitty_keys: true,
        underline_styles: true,
        sync: true,
        notices: notices(Some(Notify::Osc777), true),
    },
    Terminal {
        known: Known::Iterm,
        term: None,
        env: &[
            Signal::Is("TERM_PROGRAM", "iTerm.app"),
            Signal::Is("LC_TERMINAL", "iTerm2"),
        ],
        version: Some("iterm2"),
        kitty_keys: true,
        underline_styles: true,
        sync: true,
        notices: notices(Some(Notify::Osc9), true),
    },
    Terminal {
        known: Known::Alacritty,
        term: Some(Name::Is("alacritty")),
        env: &[Signal::Set("ALACRITTY_WINDOW_ID")],
        version: None,
        kitty_keys: true,
        underline_styles: false,
        sync: true,
        notices: notices(None, false),
    },
    Terminal {
        known: Known::WindowsTerminal,
        term: None,
        env: &[Signal::Set("WT_SESSION")],
        version: None,
        kitty_keys: false,
        underline_styles: false,
        sync: true,
        notices: notices(None, true),
    },
    Terminal {
        known: Known::Vte,
        term: None,
        env: &[Signal::Set("VTE_VERSION")],
        version: None,
        kitty_keys: false,
        underline_styles: true,
        sync: false,
        notices: notices(Some(Notify::Osc777), false),
    },
    Terminal {
        known: Known::Apple,
        term: None,
        env: &[Signal::Is("TERM_PROGRAM", "Apple_Terminal")],
        version: None,
        kitty_keys: false,
        underline_styles: false,
        sync: false,
        notices: notices(None, false),
    },
    Terminal {
        known: Known::Foot,
        term: Some(Name::Starts("foot")),
        env: &[],
        version: Some("foot"),
        kitty_keys: true,
        underline_styles: true,
        sync: true,
        notices: notices(Some(Notify::Osc777), false),
    },
    Terminal {
        known: Known::Contour,
        term: Some(Name::Starts("contour")),
        env: &[],
        version: Some("contour"),
        kitty_keys: false,
        underline_styles: true,
        sync: true,
        notices: notices(None, true),
    },
];

impl Name {
    fn matches(self, value: &str) -> bool {
        match self {
            Self::Is(name) => value == name,
            Self::Starts(prefix) => value.starts_with(prefix),
        }
    }
}

impl Signal {
    fn holds(self, env: &impl Fn(&str) -> Option<String>) -> bool {
        match self {
            Self::Set(name) => env(name).is_some(),
            Self::Is(name, value) => env(name).as_deref() == Some(value),
        }
    }
}

impl Known {
    fn terminal(self) -> Option<&'static Terminal> {
        TERMINALS.iter().find(|row| row.known == self)
    }

    pub(super) fn kitty_keys(self) -> bool {
        self.terminal().is_some_and(|row| row.kitty_keys)
    }

    pub(super) fn underline_styles(self) -> bool {
        self.terminal().is_some_and(|row| row.underline_styles)
    }

    pub(super) fn sync(self) -> bool {
        self.terminal().is_some_and(|row| row.sync)
    }

    pub(super) fn notices(self) -> Notices {
        self.terminal().map(|row| row.notices).unwrap_or_default()
    }

    #[cfg(unix)]
    pub(super) fn from_version(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        TERMINALS
            .iter()
            .find(|row| row.version.is_some_and(|prefix| name.starts_with(prefix)))
            .map(|row| row.known)
    }
}

pub(super) fn known(env: &impl Fn(&str) -> Option<String>, mux: Option<Mux>) -> Option<Known> {
    let term = env("TERM").unwrap_or_default();
    known_term(&term).or_else(|| if mux.is_some() { None } else { known_env(env) })
}

fn known_term(term: &str) -> Option<Known> {
    TERMINALS
        .iter()
        .find(|row| row.term.is_some_and(|name| name.matches(term)))
        .map(|row| row.known)
}

pub(in crate::terminal) fn known_env(env: &impl Fn(&str) -> Option<String>) -> Option<Known> {
    TERMINALS
        .iter()
        .flat_map(|row| row.env.iter().map(move |signal| (row.known, signal)))
        .find(|(_, signal)| signal.holds(env))
        .map(|(known, _)| known)
}
