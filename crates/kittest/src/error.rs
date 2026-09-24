use std::fmt;

#[derive(Clone, Debug)]
pub enum Error {
    NoAdapter(String),
    Device(String),
    Layout(String),
    Render(String),
    Readback(String),
    Unsettled { frames: u32, active: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAdapter(detail) => write!(
                f,
                "no wgpu adapter ({detail}); set SIGIL_ALLOW_NO_GPU=1 to skip GPU checks"
            ),
            Self::Device(detail) => write!(f, "wgpu device request failed: {detail}"),
            Self::Layout(detail) => write!(f, "layout failed: {detail}"),
            Self::Render(detail) => write!(f, "offscreen render failed: {detail}"),
            Self::Readback(detail) => write!(f, "pixel readback failed: {detail}"),
            Self::Unsettled { frames, active } => write!(
                f,
                "animation runtime did not settle within SETTLE_SAFETY_CAP ({frames}) frames; \
                 {active} animators remain, so a widget under test likely \
                 retargets every frame"
            ),
        }
    }
}

impl std::error::Error for Error {}
