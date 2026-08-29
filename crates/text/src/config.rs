use sigil_style::{TextLineHeight, TextStyle};

#[derive(Clone, Debug)]
pub struct TextConfig {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub line_height: f32,
}

impl TextConfig {
    #[must_use]
    pub fn text_style(&self) -> TextStyle {
        TextStyle {
            font_family: self.font_family.clone(),
            font_size: self.font_size.max(1.0),
            font_weight: self.font_weight,
            line_height: TextLineHeight::FontSizeRelative(self.line_height.max(0.1)),
            letter_spacing_em: 0.0,
            ..Default::default()
        }
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            font_family: String::from("system-ui"),
            font_size: 16.0,
            font_weight: 400,
            line_height: 1.2,
        }
    }
}
