use std::sync::{PoisonError, RwLock};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTuning {
    pub enabled: bool,
    pub snap: bool,
    pub dilation: f32,
    pub sharpen: f32,
    pub gamma_light_on_dark: f32,
    pub gamma_dark_on_light: f32,
    pub raster: bool,
}

const DEFAULT: TextTuning = TextTuning {
    enabled: true,
    snap: true,
    dilation: 0.5,
    sharpen: 1.0,
    gamma_light_on_dark: 1.0,
    gamma_dark_on_light: 3.5,
    raster: true,
};

impl Default for TextTuning {
    fn default() -> Self {
        DEFAULT
    }
}

static TUNING: RwLock<TextTuning> = RwLock::new(DEFAULT);

#[must_use]
pub fn text_tuning() -> TextTuning {
    *TUNING.read().unwrap_or_else(PoisonError::into_inner)
}

pub fn set_text_tuning(tuning: TextTuning) {
    *TUNING.write().unwrap_or_else(PoisonError::into_inner) = tuning;
}
