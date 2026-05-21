//! [`Key`] — a single keystroke (modifiers + key code).
//!
//! A keystroke is `modifiers + code`. The textual form is the
//! hyphen-separated lowercase representation used in keymap TOML:
//!
//! - `"a"` — letter
//! - `"enter"` / `"f1"` / `"pageup"` — named key
//! - `"ctrl-s"` / `"ctrl-shift-p"` — modifier + key
//! - `","` / `"/"` / `";"` — symbol key
//!
//! Modifiers are normalised on parse: `Shift-Ctrl-A` and `ctrl-shift-a`
//! parse to the same [`Key`].
//!
//! Sequences of [`Key`] (chords like `"g d"`) live one layer up in
//! [`super::config`].

use std::fmt;

/// Bitset of pressed modifier keys.
///
/// `Default` is the empty modifier set. The bit layout is opaque; use
/// [`Modifiers::with`] / [`Modifiers::contains`] / [`Modifiers::all`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers(u8);

/// One of the four supported modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::exhaustive_enums)]
pub enum Modifier {
    /// Control key (typically `Ctrl`).
    Ctrl,
    /// Shift key.
    Shift,
    /// Alt / Option key.
    Alt,
    /// Meta / Cmd / Super / Windows key.
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
    /// Empty modifier set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Return a new set with `m` added.
    #[must_use]
    pub const fn with(self, m: Modifier) -> Self {
        Self(self.0 | m.bit())
    }

    /// Whether `m` is in this set.
    #[must_use]
    pub const fn contains(self, m: Modifier) -> bool {
        (self.0 & m.bit()) != 0
    }

    /// Iterator over the modifiers in canonical order (ctrl, shift, alt, meta).
    pub fn iter(self) -> impl Iterator<Item = Modifier> {
        const ALL: [Modifier; 4] = [
            Modifier::Ctrl,
            Modifier::Shift,
            Modifier::Alt,
            Modifier::Meta,
        ];
        ALL.into_iter().filter(move |m| self.contains(*m))
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// The non-modifier part of a key event.
///
/// Letters are normalised to lowercase (case is carried by the
/// [`Modifier::Shift`] modifier). Function keys are F1..F24.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(clippy::exhaustive_enums)]
pub enum KeyCode {
    /// Single ASCII letter `a`..`z`.
    Char(char),
    /// Decimal digit `0`..`9`.
    Digit(u8),
    /// Function key (1..=24).
    Function(u8),
    /// A named non-printing key.
    Named(NamedKey),
    /// A symbol key (`,`, `.`, `/`, `;`, etc.).
    Symbol(char),
}

/// The named keys recognised by [`Key::parse`].
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

/// A single keystroke: modifiers plus the non-modifier key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    /// Modifier set held when the key was pressed.
    pub modifiers: Modifiers,
    /// The non-modifier key code.
    pub code: KeyCode,
}

/// Parse errors surfaced by [`Key::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum ParseError {
    /// The input was empty (or all whitespace).
    Empty,
    /// The input had no non-modifier token (e.g. `"ctrl-shift-"`).
    NoKey,
    /// A token in the dash-list could not be parsed as a modifier or as
    /// the trailing key code.
    UnknownToken {
        /// The unrecognised token (lowercased).
        token: String,
    },
    /// The same modifier appeared twice (`"ctrl-ctrl-a"`).
    DuplicateModifier {
        /// The modifier that was duplicated.
        modifier: Modifier,
    },
    /// More than one non-modifier token in the keystroke
    /// (`"ctrl-a-b"`). For chords (multi-key sequences) use the
    /// space-separated form in TOML, not multiple trailing tokens.
    TooManyKeys,
    /// Function-key index was outside 1..=24.
    FunctionKeyOutOfRange {
        /// The out-of-range index as written (e.g. `25`).
        n: u32,
    },
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
    /// Parse a single keystroke from its TOML-style textual form.
    ///
    /// The input is split on `-`. Every token but the last must be a
    /// modifier name (`ctrl`, `shift`, `alt`, `meta` plus aliases like
    /// `cmd`, `super`, `opt`). The last token is the key itself —
    /// a letter, digit, function key (`f1`..`f24`), named key
    /// (`enter`, `tab`, …) or symbol.
    ///
    /// Case is folded to lowercase before parsing. Surrounding
    /// whitespace is trimmed.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] for the cases enumerated on that type.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ParseError::Empty);
        }
        let lower = trimmed.to_lowercase();
        // The minus character is itself a valid key — only treat `-` as
        // a separator when it isn't the entire input.
        if lower == "-" {
            return Ok(Self {
                modifiers: Modifiers::empty(),
                code: KeyCode::Symbol('-'),
            });
        }
        super::parse::parse_tokens(&lower)
    }

    /// Construct a keystroke from explicit parts.
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
