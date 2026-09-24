use super::{Theme, ThemeValue};
use crate::style::Rgba8;

impl Theme {
    #[must_use]
    pub fn console_dark() -> Self {
        Self {
            tokens: phf::phf_map! {
                "background" => ThemeValue::Color(Rgba8 { r: 10, g: 10, b: 10, a: 255 }),
                "foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "card" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "card-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "popover" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "popover-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "primary" => ThemeValue::Color(Rgba8 { r: 229, g: 229, b: 229, a: 255 }),
                "primary-foreground" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "secondary" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "secondary-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "muted" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "muted-foreground" => ThemeValue::Color(Rgba8 { r: 161, g: 161, b: 161, a: 255 }),
                "accent" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "accent-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "destructive" => ThemeValue::Color(Rgba8 { r: 255, g: 100, b: 103, a: 255 }),
                "destructive-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "border" => ThemeValue::Color(Rgba8 { r: 245, g: 255, b: 255, a: 26 }),
                "input" => ThemeValue::Color(Rgba8 { r: 255, g: 255, b: 255, a: 38 }),
                "ring" => ThemeValue::Color(Rgba8 { r: 115, g: 115, b: 115, a: 255 }),
                "sidebar" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "sidebar-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-primary" => ThemeValue::Color(Rgba8 { r: 20, g: 71, b: 230, a: 255 }),
                "sidebar-primary-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-accent" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "sidebar-accent-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-border" => ThemeValue::Color(Rgba8 { r: 245, g: 255, b: 255, a: 26 }),
                "sidebar-ring" => ThemeValue::Color(Rgba8 { r: 115, g: 115, b: 115, a: 255 }),
                "syntax-keyword" => ThemeValue::Color(Rgba8 { r: 198, g: 120, b: 221, a: 255 }),
                "syntax-string" => ThemeValue::Color(Rgba8 { r: 152, g: 195, b: 121, a: 255 }),
                "syntax-comment" => ThemeValue::Color(Rgba8 { r: 117, g: 123, b: 138, a: 255 }),
                "syntax-function" => ThemeValue::Color(Rgba8 { r: 97, g: 175, b: 239, a: 255 }),
                "syntax-type" => ThemeValue::Color(Rgba8 { r: 229, g: 192, b: 123, a: 255 }),
                "syntax-number" => ThemeValue::Color(Rgba8 { r: 209, g: 154, b: 102, a: 255 }),
                "syntax-constant" => ThemeValue::Color(Rgba8 { r: 209, g: 154, b: 102, a: 255 }),
                "syntax-property" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
                "syntax-operator" => ThemeValue::Color(Rgba8 { r: 86, g: 182, b: 194, a: 255 }),
                "syntax-punctuation" => ThemeValue::Color(Rgba8 { r: 171, g: 178, b: 191, a: 255 }),
                "syntax-variable" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
                "syntax-tag" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
            },
            ..Self::dark()
        }
    }
}
