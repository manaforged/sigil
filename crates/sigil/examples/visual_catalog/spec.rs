use serde::{Deserialize, Serialize};
use sigil::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
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

pub(crate) type Build = fn(State) -> NodeBuilder;

pub(crate) fn off(state: State) -> Signal<bool> {
    signal(state == State::Disabled)
}

pub(crate) fn node(widget: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .child(widget)
}
