use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTuning {
    pub enabled: bool,
    pub snap: bool,
    pub dilation: f32,
    pub sharpen: f32,
    pub gamma_light_on_dark: f32,
    pub gamma_dark_on_light: f32,
}

impl Default for TextTuning {
    fn default() -> Self {
        Self {
            enabled: true,
            snap: true,
            dilation: 0.5,
            sharpen: 2.0,
            gamma_light_on_dark: 1.5,
            gamma_dark_on_light: 2.5,
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static SNAP: AtomicBool = AtomicBool::new(true);
static DILATION: AtomicU32 = AtomicU32::new(0x3F00_0000); // 0.5
static SHARPEN: AtomicU32 = AtomicU32::new(0x4000_0000); // 2.0
static GAMMA_LOD: AtomicU32 = AtomicU32::new(0x3FC0_0000); // 1.5
static GAMMA_DOL: AtomicU32 = AtomicU32::new(0x4020_0000); // 2.5

#[must_use]
pub fn text_tuning() -> TextTuning {
    TextTuning {
        enabled: ENABLED.load(Ordering::Relaxed),
        snap: SNAP.load(Ordering::Relaxed),
        dilation: f32::from_bits(DILATION.load(Ordering::Relaxed)),
        sharpen: f32::from_bits(SHARPEN.load(Ordering::Relaxed)),
        gamma_light_on_dark: f32::from_bits(GAMMA_LOD.load(Ordering::Relaxed)),
        gamma_dark_on_light: f32::from_bits(GAMMA_DOL.load(Ordering::Relaxed)),
    }
}

pub fn set_text_tuning(tuning: TextTuning) {
    ENABLED.store(tuning.enabled, Ordering::Relaxed);
    SNAP.store(tuning.snap, Ordering::Relaxed);
    DILATION.store(tuning.dilation.to_bits(), Ordering::Relaxed);
    SHARPEN.store(tuning.sharpen.to_bits(), Ordering::Relaxed);
    GAMMA_LOD.store(tuning.gamma_light_on_dark.to_bits(), Ordering::Relaxed);
    GAMMA_DOL.store(tuning.gamma_dark_on_light.to_bits(), Ordering::Relaxed);
}
