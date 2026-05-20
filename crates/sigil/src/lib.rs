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
//!
//! # `.class()` vs `.styled()`
//!
//! Per DR-049, `.class(...)` is the default styling path: it accepts a
//! static `&[StyleAtom]` slice (or a parsed class string like
//! `"p-4 bg-accent rounded-md"`) and is the right call for compile-time
//! atoms that do not change per instance. `.styled()` opens the trait
//! surface for per-method authoring — chain individual fluent methods
//! like `.padding(8).bg(token("accent"))` when an atom needs a typed,
//! per-call shape rather than a class string. Both paths funnel into
//! the same atom merger; pick `.class()` for static utility-class
//! authoring and `.styled()` when the surface benefits from typed
//! method calls.

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use image as _;
#[cfg(test)]
use pollster as _;
#[cfg(test)]
use sigil_animation as _;
#[cfg(test)]
use sigil_hal as _;
#[cfg(test)]
use sigil_renderer as _;
#[cfg(test)]
use wgpu as _;

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
    pub use sigil_actions::{Action, DeriveAction, DispatchCx, HandlerKey};
    pub use sigil_async::{OwnerHandle, dispatch_to_owner};
    pub use sigil_events::prelude::*;
    pub use sigil_kernel::prelude::*;
    pub use sigil_layout::prelude::*;
    pub use sigil_reactive::{Signal, effect, signal};
    pub use sigil_style::{
        ColorOrToken, Fill, Flex, FocusRing, Geometry, NodeStyleExt, Sizing, Spacing, StateStyle,
        StateVariants, Style, StyleAccess, StyleAtom, StyleState, StyleVariant, Styleable, Styled,
        Theme, Transitions, Typography, Visual,
    };

    pub use crate::interaction::InteractionState;
    pub use crate::widgets::{
        ButtonBuilder, ButtonTone, CheckboxBuilder, CheckboxTone, Clipboard, DropdownBuilder,
        FctSpawn, FloatingTextBuilder, MenuBuilder, MenuItem, ModalBuilder, Orientation,
        PanelBuilder, ProgressBuilder, ScrollBuilder, SliderBuilder, TabsBuilder, TextFieldBuilder,
        ToggleBuilder, TooltipBuilder, button, checkbox, dropdown, floating_text, label, menu,
        modal, panel, progress, scroll, slider, tabs, text_field, toggle, tooltip,
    };
    pub use crate::{AppConfig, Error, Result, launch};
}
