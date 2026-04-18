//! The sigil UI framework.
//!
//! Thin facade over [`sigil_app`] and the kernel crates so user code
//! writes `use sigil::prelude::*;` and never has to name individual
//! sub-crates. See DR-016 (app launch) and the ergonomics-pass plan.
//!
//! ```text
//! use sigil::prelude::*;
//!
//! fn main() -> sigil::Result<()> {
//!     sigil::launch("Demo", |cx| {
//!         cx.add(container().class("bg-accent"))
//!     })
//! }
//! ```

#[cfg(test)]
use sigil_hal as _;
#[cfg(test)]
use sigil_renderer as _;
#[cfg(test)]
use taffy as _;

pub use sigil_app::{AppConfig, Error, PanicPolicy, Result, TreeRoot, launch};

pub mod interaction;
pub mod widgets;

/// Reactive primitives re-exported unchanged.
pub mod reactive {
    pub use sigil_reactive::{
        Runtime, Signal, batch, derived, derived_eq, effect, effect_named, on_cleanup, signal,
    };
}

/// Prelude: the imports a user writes at the top of an app.
///
/// Brings in the core tree API (`container`, `text`, `fragment`,
/// `NodeBuilder`, `NodeId`, `Cx`), the layout prelude (`LayoutStyle`,
/// `Size`, `Position`), the event-handler builder surface, and the
/// style layer (`NodeStyleExt::class` / `style` / `theme` /
/// `style_reactive`). Launch + config types are at the crate root.
pub mod prelude {
    pub use sigil_core::prelude::*;
    pub use sigil_events::prelude::*;
    pub use sigil_layout::prelude::*;
    pub use sigil_reactive::{Signal, effect, signal};
    pub use sigil_style::{
        FocusRing, NodeStyleExt, StateStyle, Style, StyleAtom, StyleState, StyleVariant, Theme,
    };

    pub use crate::interaction::InteractionState;
    pub use crate::widgets::{
        ButtonBuilder, ButtonTone, CheckboxBuilder, CheckboxTone, Orientation, SliderBuilder,
        TextFieldBuilder, ToggleBuilder, button, checkbox, label, slider, text_field, toggle,
    };
    pub use crate::{AppConfig, Error, Result, launch};
}
