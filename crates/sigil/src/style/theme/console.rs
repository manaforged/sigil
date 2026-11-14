use super::Theme;
use crate::style::Color;

static TOKENS: phf::Map<&str, Color> = phf::phf_map! {
    "background" => Color::rgb(10, 10, 10),
    "foreground" => Color::rgb(250, 250, 250),
    "card" => Color::rgb(23, 23, 23),
    "card-foreground" => Color::rgb(250, 250, 250),
    "popover" => Color::rgb(23, 23, 23),
    "popover-foreground" => Color::rgb(250, 250, 250),
    "primary" => Color::rgb(229, 229, 229),
    "primary-foreground" => Color::rgb(23, 23, 23),
    "secondary" => Color::rgb(38, 38, 38),
    "secondary-foreground" => Color::rgb(250, 250, 250),
    "muted" => Color::rgb(38, 38, 38),
    "muted-foreground" => Color::rgb(161, 161, 161),
    "accent" => Color::rgb(38, 38, 38),
    "accent-foreground" => Color::rgb(250, 250, 250),
    "destructive" => Color::rgb(255, 100, 103),
    "destructive-foreground" => Color::rgb(250, 250, 250),
    "border" => Color { r: 245, g: 255, b: 255, a: 26 },
    "input" => Color { r: 255, g: 255, b: 255, a: 38 },
    "ring" => Color::rgb(115, 115, 115),
    "sidebar" => Color::rgb(23, 23, 23),
    "sidebar-foreground" => Color::rgb(250, 250, 250),
    "sidebar-primary" => Color::rgb(20, 71, 230),
    "sidebar-primary-foreground" => Color::rgb(250, 250, 250),
    "sidebar-accent" => Color::rgb(38, 38, 38),
    "sidebar-accent-foreground" => Color::rgb(250, 250, 250),
    "sidebar-border" => Color { r: 245, g: 255, b: 255, a: 26 },
    "sidebar-ring" => Color::rgb(115, 115, 115),
    "syntax-keyword" => Color::rgb(198, 120, 221),
    "syntax-string" => Color::rgb(152, 195, 121),
    "syntax-comment" => Color::rgb(117, 123, 138),
    "syntax-function" => Color::rgb(97, 175, 239),
    "syntax-type" => Color::rgb(229, 192, 123),
    "syntax-number" => Color::rgb(209, 154, 102),
    "syntax-constant" => Color::rgb(209, 154, 102),
    "syntax-property" => Color::rgb(224, 108, 117),
    "syntax-operator" => Color::rgb(86, 182, 194),
    "syntax-punctuation" => Color::rgb(171, 178, 191),
    "syntax-variable" => Color::rgb(224, 108, 117),
    "syntax-tag" => Color::rgb(224, 108, 117),
};

impl Theme {
    /// Creates the dark theme with the console token overrides.
    #[must_use]
    pub fn console_dark() -> Self {
        Self::dark().theme_tokens(&TOKENS)
    }
}
