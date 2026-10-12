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

#[cfg(unix)]
const XTVERSION: [(&str, Known); 6] = [
    ("kitty", Known::Kitty),
    ("ghostty", Known::Ghostty),
    ("wezterm", Known::WezTerm),
    ("iterm2", Known::Iterm),
    ("foot", Known::Foot),
    ("contour", Known::Contour),
];

impl Known {
    pub(super) const fn kitty_keys(self) -> bool {
        match self {
            Self::Kitty
            | Self::Ghostty
            | Self::WezTerm
            | Self::Iterm
            | Self::Foot
            | Self::Alacritty => true,
            Self::Contour | Self::WindowsTerminal | Self::Vte | Self::Apple => false,
        }
    }

    pub(super) const fn underline_styles(self) -> bool {
        match self {
            Self::Apple | Self::Alacritty | Self::WindowsTerminal => false,
            Self::Kitty
            | Self::Ghostty
            | Self::WezTerm
            | Self::Iterm
            | Self::Foot
            | Self::Contour
            | Self::Vte => true,
        }
    }

    pub(super) const fn notices(self) -> Notices {
        let (notify, progress) = match self {
            Self::Kitty | Self::Iterm => (Some(Notify::Osc9), true),
            Self::Ghostty | Self::WezTerm => (Some(Notify::Osc777), true),
            Self::Foot | Self::Vte => (Some(Notify::Osc777), false),
            Self::WindowsTerminal | Self::Contour => (None, true),
            Self::Alacritty | Self::Apple => (None, false),
        };
        Notices { notify, progress }
    }

    #[cfg(unix)]
    pub(super) fn from_version(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        XTVERSION
            .iter()
            .find(|(prefix, _)| name.starts_with(prefix))
            .map(|(_, known)| *known)
    }
}

pub(super) fn known(env: &impl Fn(&str) -> Option<String>, mux: Option<Mux>) -> Option<Known> {
    let term = env("TERM").unwrap_or_default();
    known_term(&term).or_else(|| if mux.is_some() { None } else { known_env(env) })
}

fn known_term(term: &str) -> Option<Known> {
    Some(match term {
        "xterm-kitty" => Known::Kitty,
        "xterm-ghostty" => Known::Ghostty,
        "wezterm" => Known::WezTerm,
        "alacritty" => Known::Alacritty,
        _ if term.starts_with("foot") => Known::Foot,
        _ if term.starts_with("contour") => Known::Contour,
        _ => return None,
    })
}

pub(in crate::terminal) fn known_env(env: &impl Fn(&str) -> Option<String>) -> Option<Known> {
    let program = env("TERM_PROGRAM").unwrap_or_default();
    Some(if env("KITTY_WINDOW_ID").is_some() {
        Known::Kitty
    } else if env("GHOSTTY_RESOURCES_DIR").is_some() || program == "ghostty" {
        Known::Ghostty
    } else if env("WEZTERM_EXECUTABLE").is_some() || program == "WezTerm" {
        Known::WezTerm
    } else if program == "iTerm.app" || env("LC_TERMINAL").as_deref() == Some("iTerm2") {
        Known::Iterm
    } else if env("ALACRITTY_WINDOW_ID").is_some() {
        Known::Alacritty
    } else if env("WT_SESSION").is_some() {
        Known::WindowsTerminal
    } else if env("VTE_VERSION").is_some() {
        Known::Vte
    } else if program == "Apple_Terminal" {
        Known::Apple
    } else {
        return None;
    })
}
