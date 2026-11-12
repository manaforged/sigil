use super::{Theme, ThemeValue};
use crate::style::Color;

static TOKENS: phf::Map<&str, ThemeValue> = phf::phf_map! {
    "background" => ThemeValue::Color(Color::rgb(10, 10, 10)),
    "foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "card" => ThemeValue::Color(Color::rgb(23, 23, 23)),
    "card-foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "popover" => ThemeValue::Color(Color::rgb(23, 23, 23)),
    "popover-foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "primary" => ThemeValue::Color(Color::rgb(229, 229, 229)),
    "primary-foreground" => ThemeValue::Color(Color::rgb(23, 23, 23)),
    "secondary" => ThemeValue::Color(Color::rgb(38, 38, 38)),
    "secondary-foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "muted" => ThemeValue::Color(Color::rgb(38, 38, 38)),
    "muted-foreground" => ThemeValue::Color(Color::rgb(161, 161, 161)),
    "accent" => ThemeValue::Color(Color::rgb(38, 38, 38)),
    "accent-foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "destructive" => ThemeValue::Color(Color::rgb(255, 100, 103)),
    "destructive-foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "border" => ThemeValue::Color(Color { r: 245, g: 255, b: 255, a: 26 }),
    "input" => ThemeValue::Color(Color { r: 255, g: 255, b: 255, a: 38 }),
    "ring" => ThemeValue::Color(Color::rgb(115, 115, 115)),
    "sidebar" => ThemeValue::Color(Color::rgb(23, 23, 23)),
    "sidebar-foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "sidebar-primary" => ThemeValue::Color(Color::rgb(20, 71, 230)),
    "sidebar-primary-foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "sidebar-accent" => ThemeValue::Color(Color::rgb(38, 38, 38)),
    "sidebar-accent-foreground" => ThemeValue::Color(Color::rgb(250, 250, 250)),
    "sidebar-border" => ThemeValue::Color(Color { r: 245, g: 255, b: 255, a: 26 }),
    "sidebar-ring" => ThemeValue::Color(Color::rgb(115, 115, 115)),
    "syntax-keyword" => ThemeValue::Color(Color::rgb(198, 120, 221)),
    "syntax-string" => ThemeValue::Color(Color::rgb(152, 195, 121)),
    "syntax-comment" => ThemeValue::Color(Color::rgb(117, 123, 138)),
    "syntax-function" => ThemeValue::Color(Color::rgb(97, 175, 239)),
    "syntax-type" => ThemeValue::Color(Color::rgb(229, 192, 123)),
    "syntax-number" => ThemeValue::Color(Color::rgb(209, 154, 102)),
    "syntax-constant" => ThemeValue::Color(Color::rgb(209, 154, 102)),
    "syntax-property" => ThemeValue::Color(Color::rgb(224, 108, 117)),
    "syntax-operator" => ThemeValue::Color(Color::rgb(86, 182, 194)),
    "syntax-punctuation" => ThemeValue::Color(Color::rgb(171, 178, 191)),
    "syntax-variable" => ThemeValue::Color(Color::rgb(224, 108, 117)),
    "syntax-tag" => ThemeValue::Color(Color::rgb(224, 108, 117)),
};

impl Theme {
    /// Creates the dark theme with the console token overrides.
    #[must_use]
    pub fn console_dark() -> Self {
        Self::dark().theme_tokens(&TOKENS)
    }
}
