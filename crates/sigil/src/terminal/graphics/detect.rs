use crate::terminal::caps::{Known, known_env};

/// A terminal image protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Protocol {
    /// The kitty protocol.
    Kitty,
    /// The iterm2 protocol.
    Iterm2,
    /// The sixel protocol.
    Sixel,
}

/// A terminal multiplexer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mux {
    /// The tmux mux.
    Tmux,
    /// The screen mux.
    Screen,
}

/// Detects a multiplexer from environment variables that `env` reads.
#[must_use]
pub(crate) fn detect_mux(env: impl Fn(&str) -> Option<String>) -> Option<Mux> {
    if env("TMUX").is_some() {
        Some(Mux::Tmux)
    } else if env("STY").is_some() {
        Some(Mux::Screen)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Forced {
    Off,
    On(Protocol),
}

impl std::str::FromStr for Forced {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" | "off" => Ok(Self::Off),
            "kitty" => Ok(Self::On(Protocol::Kitty)),
            "iterm2" | "iterm" => Ok(Self::On(Protocol::Iterm2)),
            "sixel" => Ok(Self::On(Protocol::Sixel)),
            _ => Err(()),
        }
    }
}

fn forced_graphics(env: &impl Fn(&str) -> Option<String>) -> Option<Forced> {
    env("SIGIL_GRAPHICS")?.parse().ok()
}

/// Detects an image protocol from environment variables that `env` reads.
pub fn detect(env: impl Fn(&str) -> Option<String>) -> Option<Protocol> {
    match forced_graphics(&env) {
        Some(Forced::Off) => return None,
        Some(Forced::On(protocol)) => return Some(protocol),
        None => {}
    }
    let term = env("TERM").unwrap_or_default();
    let program = env("TERM_PROGRAM").unwrap_or_default();
    if env("KONSOLE_VERSION").is_some() || matches!(term.as_str(), "xterm-kitty" | "xterm-ghostty")
    {
        return Some(Protocol::Kitty);
    }
    match known_env(&env) {
        Some(Known::Kitty | Known::Ghostty) => return Some(Protocol::Kitty),
        Some(Known::WezTerm | Known::Iterm) => return Some(Protocol::Iterm2),
        _ => {}
    }
    if program == "WarpTerminal" {
        return Some(Protocol::Iterm2);
    }
    if program == "herdr" {
        return Some(Protocol::Kitty);
    }
    if sixel_capable(&term, &program) {
        return Some(Protocol::Sixel);
    }
    None
}

fn sixel_capable(term: &str, program: &str) -> bool {
    let base = term.split('-').next().unwrap_or_default();
    term.contains("sixel")
        || matches!(base, "foot" | "contour" | "mlterm" | "yaft" | "rio")
        || ["contour", "mintty", "mlterm", "rio", "foot"]
            .iter()
            .any(|name| program.eq_ignore_ascii_case(name))
}
