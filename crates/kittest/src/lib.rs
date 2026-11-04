#![doc = include_str!("../README.md")]
use sigil::layout::TextMeasure;

mod bounds;
mod compare;
pub use compare::Golden;
pub use sigil::kernel::WithCx;
mod error;
mod explore;
pub use explore::Step;
pub use error::Error;
pub mod gpu;
mod harness;
mod render;
pub use harness::{Harness, Node, Query, Target};
pub use render::Render;
mod interaction;
pub mod parity;
pub mod scene_gen;
mod settle;
pub use sigil::layout::FakeMeasure;

pub const SETTLE_SAFETY_CAP: u32 = 600;

pub const EXPECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[allow(clippy::cast_precision_loss)]
pub(crate) const fn extent_f32(value: u32) -> f32 {
    value as f32
}

pub type Rendered = Result<Option<image::RgbaImage>, Error>;

#[derive(Clone, Copy, Debug)]
pub enum InteractionMode {
    Default,
    Hover,
    Pressed,
    Clicked,
    Focused,
    Disabled,
    HoverAfterFrames { frames: u32, dt: f32 },
    PressedAfterFrames { frames: u32, dt: f32 },
    FocusedAfterFrames { frames: u32, dt: f32 },
    Dragged { dx: f32, dy: f32 },
}

impl InteractionMode {
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Hover | Self::HoverAfterFrames { .. } => "hover",
            Self::Pressed | Self::PressedAfterFrames { .. } => "pressed",
            Self::Clicked => "clicked",
            Self::Focused | Self::FocusedAfterFrames { .. } => "focused",
            Self::Disabled => "disabled",
            Self::Dragged { .. } => "dragged",
        }
    }
}

#[must_use]
pub fn stub_text_measure() -> Box<dyn TextMeasure> {
    Box::new(FakeMeasure::fixed(40.0, 16.0))
}

#[must_use]
pub fn real_text_measure() -> Box<dyn TextMeasure> {
    Box::new(sigil::app::DefaultTextMeasure::new())
}

#[cfg(doctest)]
#[doc = include_str!("../../../docs/guide/automation.md")]
pub struct AutomationGuide;

#[cfg(doctest)]
#[doc = include_str!("../../../docs/guide/testing.md")]
pub struct TestingGuide;
