use std::fmt;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::exhaustive_enums)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Meta,
}

impl Modifier {
    const fn bit(self) -> u8 {
        match self {
            Self::Ctrl => 1 << 0,
            Self::Shift => 1 << 1,
            Self::Alt => 1 << 2,
            Self::Meta => 1 << 3,
        }
    }

    pub(super) fn parse(token: &str) -> Option<Self> {
        match token {
            "ctrl" | "control" => Some(Self::Ctrl),
            "shift" => Some(Self::Shift),
            "alt" | "opt" | "option" => Some(Self::Alt),
            "meta" | "cmd" | "super" | "win" => Some(Self::Meta),
            _ => None,
        }
    }

    pub(super) const fn token(self) -> &'static str {
        match self {
            Self::Ctrl => "ctrl",
            Self::Shift => "shift",
            Self::Alt => "alt",
            Self::Meta => "meta",
        }
    }
}

impl Modifiers {
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn with(self, m: Modifier) -> Self {
        Self(self.0 | m.bit())
    }

    #[must_use]
    pub const fn contains(self, m: Modifier) -> bool {
        (self.0 & m.bit()) != 0
    }

    pub fn iter(self) -> impl Iterator<Item = Modifier> {
        const ALL: [Modifier; 4] = [
            Modifier::Ctrl,
            Modifier::Shift,
            Modifier::Alt,
            Modifier::Meta,
        ];
        ALL.into_iter().filter(move |m| self.contains(*m))
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(clippy::exhaustive_enums)]
pub enum KeyCode {
    Char(char),
    Digit(u8),
    Function(u8),
    Named(NamedKey),
    Symbol(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs, clippy::exhaustive_enums)]
pub enum NamedKey {
    Enter,
    Tab,
    Escape,
    Space,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
}

impl NamedKey {
    pub(super) fn parse(s: &str) -> Option<Self> {
        match s {
            "enter" | "return" => Some(Self::Enter),
            "tab" => Some(Self::Tab),
            "escape" | "esc" => Some(Self::Escape),
            "space" => Some(Self::Space),
            "backspace" => Some(Self::Backspace),
            "delete" | "del" => Some(Self::Delete),
            "home" => Some(Self::Home),
            "end" => Some(Self::End),
            "pageup" | "pgup" => Some(Self::PageUp),
            "pagedown" | "pgdn" => Some(Self::PageDown),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }

    const fn token(self) -> &'static str {
        match self {
            Self::Enter => "enter",
            Self::Tab => "tab",
            Self::Escape => "escape",
            Self::Space => "space",
            Self::Backspace => "backspace",
            Self::Delete => "delete",
            Self::Home => "home",
            Self::End => "end",
            Self::PageUp => "pageup",
            Self::PageDown => "pagedown",
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub modifiers: Modifiers,
    pub code: KeyCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum ParseError {
    Empty,
    NoKey,
    UnknownToken { token: String },
    DuplicateModifier { modifier: Modifier },
    TooManyKeys,
    FunctionKeyOutOfRange { n: u32 },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty key string"),
            Self::NoKey => f.write_str("modifier without trailing key"),
            Self::UnknownToken { token } => write!(f, "unknown key token '{token}'"),
            Self::DuplicateModifier { modifier } => {
                write!(f, "modifier '{}' specified twice", modifier.token())
            }
            Self::TooManyKeys => f.write_str("more than one non-modifier key in keystroke"),
            Self::FunctionKeyOutOfRange { n } => {
                write!(f, "function key F{n} out of range (F1..=F24)")
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl Key {
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ParseError::Empty);
        }
        let lower = trimmed.to_lowercase();
        if lower == "-" {
            return Ok(Self {
                modifiers: Modifiers::empty(),
                code: KeyCode::Symbol('-'),
            });
        }
        super::parse::parse_tokens(&lower)
    }

    #[must_use]
    pub const fn new(modifiers: Modifiers, code: KeyCode) -> Self {
        Self { modifiers, code }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for m in self.modifiers.iter() {
            write!(f, "{}-", m.token())?;
        }
        match &self.code {
            KeyCode::Char(c) | KeyCode::Symbol(c) => write!(f, "{c}"),
            KeyCode::Digit(d) => write!(f, "{d}"),
            KeyCode::Function(n) => write!(f, "f{n}"),
            KeyCode::Named(n) => f.write_str(n.token()),
        }
    }
}
