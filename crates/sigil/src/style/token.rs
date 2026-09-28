use crate::style::scale::ScaleRole;
use crate::style::{ColorAtom, StyleAtom};

macro_rules! tokens {
    ($($variant:ident => $name:literal,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum Token {
            $($variant,)*
        }

        impl Token {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];

            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

tokens! {
    Accent => "accent",
    AccentHover => "accent-hover",
    OnAccent => "on-accent",
    OnSolid => "on-solid",
    Field => "field",
    Surface => "surface",
    SurfaceHover => "surface-hover",
    SurfacePressed => "surface-pressed",
    SurfacePressedStrong => "surface-pressed-strong",
    SurfaceSubtle => "surface-subtle",
    SurfaceInset => "surface-inset",
    OnSurface => "on-surface",
    OnSurfaceMuted => "on-surface-muted",
    Border => "border",
    BorderStrong => "border-strong",
    BorderSubtle => "border-subtle",
    TableHeader => "table-header",
    Scrim => "scrim",
    Ring => "ring",
    Overlay => "overlay",
    OverlayBorder => "overlay-border",
    Gray1 => "gray-1",
    Gray2 => "gray-2",
    Gray3 => "gray-3",
    Gray4 => "gray-4",
    Gray5 => "gray-5",
    Gray6 => "gray-6",
    Gray7 => "gray-7",
    Gray8 => "gray-8",
    Gray9 => "gray-9",
    Gray10 => "gray-10",
    Gray11 => "gray-11",
    Gray12 => "gray-12",
    Accent1 => "accent-1",
    Accent2 => "accent-2",
    Accent3 => "accent-3",
    Accent4 => "accent-4",
    Accent5 => "accent-5",
    Accent6 => "accent-6",
    Accent7 => "accent-7",
    Accent8 => "accent-8",
    Accent9 => "accent-9",
    Accent10 => "accent-10",
    Accent11 => "accent-11",
    Accent12 => "accent-12",
    Success1 => "success-1",
    Success2 => "success-2",
    Success3 => "success-3",
    Success4 => "success-4",
    Success5 => "success-5",
    Success6 => "success-6",
    Success7 => "success-7",
    Success8 => "success-8",
    Success9 => "success-9",
    Success10 => "success-10",
    Success11 => "success-11",
    Success12 => "success-12",
    Warn1 => "warn-1",
    Warn2 => "warn-2",
    Warn3 => "warn-3",
    Warn4 => "warn-4",
    Warn5 => "warn-5",
    Warn6 => "warn-6",
    Warn7 => "warn-7",
    Warn8 => "warn-8",
    Warn9 => "warn-9",
    Warn10 => "warn-10",
    Warn11 => "warn-11",
    Warn12 => "warn-12",
    Error1 => "error-1",
    Error2 => "error-2",
    Error3 => "error-3",
    Error4 => "error-4",
    Error5 => "error-5",
    Error6 => "error-6",
    Error7 => "error-7",
    Error8 => "error-8",
    Error9 => "error-9",
    Error10 => "error-10",
    Error11 => "error-11",
    Error12 => "error-12",
    Info1 => "info-1",
    Info2 => "info-2",
    Info3 => "info-3",
    Info4 => "info-4",
    Info5 => "info-5",
    Info6 => "info-6",
    Info7 => "info-7",
    Info8 => "info-8",
    Info9 => "info-9",
    Info10 => "info-10",
    Info11 => "info-11",
    Info12 => "info-12",
}

const SEMANTIC: usize = 21;
const STEPS: usize = 12;
const ROLES: [ScaleRole; 6] = [
    ScaleRole::Gray,
    ScaleRole::Accent,
    ScaleRole::Success,
    ScaleRole::Warn,
    ScaleRole::Error,
    ScaleRole::Info,
];

impl Token {
    pub const COUNT: usize = Self::ALL.len();

    #[must_use]
    pub fn scale(role: ScaleRole, step: u8) -> Option<Self> {
        let offset = usize::from(step.checked_sub(1)?);
        if offset >= STEPS {
            return None;
        }
        let row = ROLES.iter().position(|r| *r == role)?;
        Self::ALL.get(SEMANTIC + row * STEPS + offset).copied()
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[must_use]
    pub fn scale_step(self) -> Option<(ScaleRole, u8)> {
        let offset = self.index().checked_sub(SEMANTIC)?;
        let role = *ROLES.get(offset / STEPS)?;
        let step = u8::try_from(offset % STEPS + 1).ok()?;
        Some((role, step))
    }

    #[must_use]
    pub const fn color(self) -> ColorAtom {
        ColorAtom::ThemeRef(ColorKey::Token(self))
    }

    #[must_use]
    pub const fn bg(self) -> StyleAtom {
        StyleAtom::BackgroundThemeRef(ColorKey::Token(self))
    }

    #[must_use]
    pub const fn text(self) -> StyleAtom {
        StyleAtom::TextThemeRef(ColorKey::Token(self))
    }

    #[must_use]
    pub const fn border(self) -> StyleAtom {
        StyleAtom::BorderThemeRef(ColorKey::Token(self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorKey {
    Token(Token),
    Custom(&'static str),
}

impl ColorKey {
    #[must_use]
    pub fn parse(name: &'static str) -> Self {
        Token::from_name(name).map_or(Self::Custom(name), Self::Token)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Token(token) => token.name(),
            Self::Custom(name) => name,
        }
    }
}

impl From<Token> for ColorKey {
    fn from(token: Token) -> Self {
        Self::Token(token)
    }
}

impl From<&'static str> for ColorKey {
    fn from(name: &'static str) -> Self {
        Self::parse(name)
    }
}
