//! Widget-author prelude for implementing built-in and extension widgets.

pub use animation_runtime::presets;
pub use events::prelude::{Key, NodeEventsExt};
pub use kernel::{Cx, IntoNode, NodeBuilder, Role, TextBuilder, container, text};
pub use layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
pub use style::{
    ClassSource, NodeStyleExt, Size, StateHost, StateStyle, Style, StyleAccess, StyleAtom,
    StyleVariant, Theme, TransitionId,
};
