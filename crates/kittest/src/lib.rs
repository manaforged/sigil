#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
use sigil::layout::TextMeasure;

mod bounds;
mod compare;
pub use compare::Golden;
pub use sigil::kernel::WithCx;
mod error;
mod explore;
pub use error::{Error, ErrorKind};
pub use explore::Step;
#[doc(hidden)]
pub mod gpu;
mod harness;
mod render;
pub use harness::{Harness, Node, Query, Target};
pub use render::Render;
mod interaction;
#[doc(hidden)]
pub mod parity;
#[doc(hidden)]
pub mod scene_gen;
mod settle;
pub use sigil::layout::FakeMeasure;

/// Most frames a harness runs while it waits for the UI to settle.
pub const SETTLE_SAFETY_CAP: u32 = 600;

/// Longest wait for a node that a harness step expects.
pub const EXPECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[allow(clippy::cast_precision_loss)]
pub(crate) const fn extent_f32(value: u32) -> f32 {
    value as f32
}

/// A rendered image, or `None` when no GPU adapter exists and skipping is allowed.
pub type Rendered = Result<Option<image::RgbaImage>, Error>;

/// Interaction state that a `Render` drives the widget into.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum InteractionMode {
    /// No interaction.
    Default,
    /// The pointer is over the widget.
    Hover,
    /// The pointer button is held down on the widget.
    Pressed,
    /// The widget was clicked: pressed and released.
    Clicked,
    /// The widget has keyboard focus.
    Focused,
    /// The widget is disabled.
    Disabled,
    /// Hovered, then `frames` frames run `dt` seconds apart.
    HoverAfterFrames {
        /// The number of frames to run.
        frames: u32,
        /// The seconds between frames.
        dt: f32,
    },
    /// Pressed, then `frames` frames run `dt` seconds apart.
    PressedAfterFrames {
        /// The number of frames to run.
        frames: u32,
        /// The seconds between frames.
        dt: f32,
    },
    /// Focused, then `frames` frames run `dt` seconds apart.
    FocusedAfterFrames {
        /// The number of frames to run.
        frames: u32,
        /// The seconds between frames.
        dt: f32,
    },
    /// Dragged from the pointer position by `dx` and `dy` logical pixels.
    Dragged {
        /// The horizontal move in logical pixels.
        dx: f32,
        /// The vertical move in logical pixels.
        dy: f32,
    },
}

impl InteractionMode {
    /// The name of the state in golden file names, such as `hover`.
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

/// A text measure that reports every text node as a 40 by 16 box.
#[must_use]
pub fn stub_text_measure() -> Box<dyn TextMeasure> {
    Box::new(FakeMeasure::fixed(40.0, 16.0))
}

/// The text measure of the app: the real text engine with the bundled fonts.
#[must_use]
pub fn real_text_measure() -> Box<dyn TextMeasure> {
    Box::new(sigil::layout::TextEngineMeasure::default())
}

#[cfg(doctest)]
#[doc = include_str!("../../../docs/guide/automation.md")]
#[derive(Debug)]
pub struct AutomationGuide;

#[cfg(doctest)]
#[doc = include_str!("../../../docs/guide/testing.md")]
#[derive(Debug)]
pub struct TestingGuide;
