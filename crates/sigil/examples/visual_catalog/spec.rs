use serde::Deserialize;
use sigil::prelude::*;
use sigil_kittest::InteractionMode;

const DT: f32 = 1.0 / 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum State {
    Default,
    Hover,
    Pressed,
    Focused,
    Disabled,
    Open,
    Shown,
    Invalid,
    Selected,
    Dragged,
}

impl State {
    pub(crate) const fn mode(self, drag: [f32; 2]) -> InteractionMode {
        match self {
            Self::Dragged => InteractionMode::Dragged {
                dx: drag[0],
                dy: drag[1],
            },
            Self::Hover => InteractionMode::HoverAfterFrames { frames: 60, dt: DT },
            Self::Pressed => InteractionMode::PressedAfterFrames { frames: 60, dt: DT },
            Self::Focused => InteractionMode::FocusedAfterFrames { frames: 60, dt: DT },
            Self::Open => InteractionMode::Clicked,
            Self::Default | Self::Disabled | Self::Shown | Self::Invalid | Self::Selected => {
                InteractionMode::Default
            }
        }
    }
}

pub(crate) type Build = fn(State) -> NodeBuilder;

pub(crate) fn off(state: State) -> Signal<bool> {
    signal(state == State::Disabled)
}

pub(crate) fn node(widget: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .child(widget)
}
