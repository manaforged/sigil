use crate::terminal::graphics::Mux;

/// The desktop notification protocol a terminal speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Notify {
    /// The osc9 notify.
    Osc9,
    /// The osc777 notify.
    Osc777,
}

/// The startup notices about missing terminal features.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Notices {
    pub(crate) notify: Option<Notify>,
    pub(crate) progress: bool,
    pub(crate) activity: bool,
}

fields!(Notices {
    "whether the terminal shows task progress" progress set_progress: bool,
    "whether the terminal shows agent activity" activity set_activity: bool,
} {
    "the desktop notification protocol" notify set_notify: Notify,
});

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
enum EnvSignal {
    Set(&'static str),
    Is(&'static str, &'static str),
}

struct Terminal {
    known: Known,
    term: Option<Name>,
    env: &'static [EnvSignal],
    version: Option<&'static str>,
    kitty_keys: bool,
    underline_styles: bool,
    sync: bool,
    notices: Notices,
}

const fn notices(notify: Option<Notify>, progress: bool) -> Notices {
    Notices {
        notify,
        progress,
        activity: false,
    }
}

const TERMINALS: [Terminal; 10] = [
    Terminal {
        known: Known::Kitty,
        term: Some(Name::Is("xterm-kitty")),
        env: &[EnvSignal::Set("KITTY_WINDOW_ID")],
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
            EnvSignal::Set("GHOSTTY_RESOURCES_DIR"),
            EnvSignal::Is("TERM_PROGRAM", "ghostty"),
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
            EnvSignal::Set("WEZTERM_EXECUTABLE"),
            EnvSignal::Is("TERM_PROGRAM", "WezTerm"),
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
            EnvSignal::Is("TERM_PROGRAM", "iTerm.app"),
            EnvSignal::Is("LC_TERMINAL", "iTerm2"),
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
        env: &[EnvSignal::Set("ALACRITTY_WINDOW_ID")],
        version: None,
        kitty_keys: true,
        underline_styles: false,
        sync: true,
        notices: notices(None, false),
    },
    Terminal {
        known: Known::WindowsTerminal,
        term: None,
        env: &[EnvSignal::Set("WT_SESSION")],
        version: None,
        kitty_keys: false,
        underline_styles: false,
        sync: true,
        notices: notices(None, true),
    },
    Terminal {
        known: Known::Vte,
        term: None,
        env: &[EnvSignal::Set("VTE_VERSION")],
        version: None,
        kitty_keys: false,
        underline_styles: true,
        sync: false,
        notices: notices(Some(Notify::Osc777), false),
    },
    Terminal {
        known: Known::Apple,
        term: None,
        env: &[EnvSignal::Is("TERM_PROGRAM", "Apple_Terminal")],
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

impl EnvSignal {
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
