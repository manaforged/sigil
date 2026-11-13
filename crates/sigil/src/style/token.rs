use crate::style::{ColorValue, StyleAtom, Tone};

macro_rules! all_tokens {
    ($($variant:ident,)*) => {
        all_tokens!(@rows [$(Token::$variant,)*] Neutral, Primary, Success, Warning, Error, Info)
    };
    (@rows [$($acc:expr,)*] $tone:ident $(, $rest:ident)*) => {
        all_tokens!(
            @rows
            [
                $($acc,)*
                Token::Step(Tone::$tone, 1), Token::Step(Tone::$tone, 2), Token::Step(Tone::$tone, 3),
                Token::Step(Tone::$tone, 4), Token::Step(Tone::$tone, 5), Token::Step(Tone::$tone, 6),
                Token::Step(Tone::$tone, 7), Token::Step(Tone::$tone, 8), Token::Step(Tone::$tone, 9),
                Token::Step(Tone::$tone, 10), Token::Step(Tone::$tone, 11), Token::Step(Tone::$tone, 12),
            ]
            $($rest),*
        )
    };
    (@rows [$($acc:expr,)*]) => {
        [$($acc,)*]
    };
}

macro_rules! step_names {
    ($prefix:literal, $step:expr) => {
        match $step {
            1 => concat!($prefix, "-1"),
            2 => concat!($prefix, "-2"),
            3 => concat!($prefix, "-3"),
            4 => concat!($prefix, "-4"),
            5 => concat!($prefix, "-5"),
            6 => concat!($prefix, "-6"),
            7 => concat!($prefix, "-7"),
            8 => concat!($prefix, "-8"),
            9 => concat!($prefix, "-9"),
            10 => concat!($prefix, "-10"),
            11 => concat!($prefix, "-11"),
            _ => concat!($prefix, "-12"),
        }
    };
}

const fn step_name(tone: Tone, step: u8) -> &'static str {
    let step = if step < 1 {
        1
    } else if step > 12 {
        12
    } else {
        step
    };
    match tone {
        Tone::Neutral => step_names!("gray", step),
        Tone::Primary => step_names!("accent", step),
        Tone::Success => step_names!("success", step),
        Tone::Warning => step_names!("warn", step),
        Tone::Error => step_names!("error", step),
        Tone::Info => step_names!("info", step),
    }
}

macro_rules! tokens {
    ($($variant:ident => $name:literal,)*) => {
        /// A named color in the theme.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum Token {
            $(#[doc = concat!("The `", $name, "` color.")] $variant,)*
            /// Step `1..=12` of a theme tone scale.
            Step(Tone, u8),
        }

        #[repr(usize)]
        enum SemanticIndex {
            $($variant,)*
        }

        const SEMANTIC_COUNT: usize = 22;

        const ALL_TOKENS: [Token; 94] = all_tokens!($($variant,)*);

        impl Token {
            /// The number of built-in tokens, the length of `ALL`.
            pub const COUNT: usize = 94;

            pub(crate) const ALL: &'static [Self; 94] = &ALL_TOKENS;

            /// Returns the position of this token in `Token::ALL`.
            #[must_use]
            pub const fn index(self) -> usize {
                match self {
                    $(Self::$variant => SemanticIndex::$variant as usize,)*
                    Self::Step(tone, step) => {
                        let row = match tone {
                            Tone::Neutral => 0,
                            Tone::Primary => 1,
                            Tone::Success => 2,
                            Tone::Warning => 3,
                            Tone::Error => 4,
                            Tone::Info => 5,
                        };
                        let clamped = if step < 1 {
                            0
                        } else if step > 12 {
                            11
                        } else {
                            (step - 1) as usize
                        };
                        SEMANTIC_COUNT + row * STEPS + clamped
                    }
                }
            }

            /// The name of this token in a theme file.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                    Self::Step(tone, step) => step_name(tone, step),
                }
            }

            /// Parses a token name from a theme file.
            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant),)*
                    _ => parse_step_name(name),
                }
            }
        }
    };
}

fn parse_step_name(name: &str) -> Option<Token> {
    let (prefix, step) = name.rsplit_once('-')?;
    let tone = match prefix {
        "gray" => Tone::Neutral,
        "accent" => Tone::Primary,
        "success" => Tone::Success,
        "warn" => Tone::Warning,
        "error" => Tone::Error,
        "info" => Tone::Info,
        _ => return None,
    };
    if step.is_empty() || step.starts_with('0') {
        return None;
    }
    let step = step.parse::<u8>().ok()?;
    (1..=12).contains(&step).then_some(Token::Step(tone, step))
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
    Placeholder => "placeholder",
    Border => "border",
    BorderStrong => "border-strong",
    BorderSubtle => "border-subtle",
    TableHeader => "table-header",
    Scrim => "scrim",
    Ring => "ring",
    Overlay => "overlay",
    OverlayBorder => "overlay-border",
}

const STEPS: usize = 12;

impl Token {
    /// Returns this token as a color atom.
    #[must_use]
    pub const fn color(self) -> ColorValue {
        ColorValue::Token(self)
    }

    #[must_use]
    pub(crate) const fn background(self) -> StyleAtom {
        StyleAtom::BackgroundColor(ColorValue::Token(self))
    }

    #[must_use]
    pub(crate) const fn text(self) -> StyleAtom {
        StyleAtom::TextColor(ColorValue::Token(self))
    }

    #[must_use]
    pub(crate) const fn border(self) -> StyleAtom {
        StyleAtom::BorderColor(ColorValue::Token(self))
    }
}
