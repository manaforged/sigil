//! The sigil UI framework.
//!
//! Thin facade over [`app`] and the kernel crates so user code
//! writes `use sigil::prelude::*;` and never has to name individual
//! sub-crates. See DR-016 (app launch) and the ergonomics-pass plan.
//!
//! ```text
//! use sigil::prelude::*;
//!
//! fn main() -> sigil::AppResult<()> {
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

#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

extern crate self as sigil;

#[cfg(test)]
use animation as _;
#[cfg(test)]
use criterion as _;
#[cfg(test)]
use hal as _;
#[cfg(test)]
use image as _;
#[cfg(test)]
use pollster as _;
#[cfg(test)]
use renderer as _;
#[cfg(test)]
use wgpu as _;

pub use app::{
    AppConfig, AppResult, DefaultFonts, Error, PanicPolicy, TextTuning, TreeRoot, launch,
    set_default_fonts, set_text_tuning, text_tuning,
};

pub mod clipboard;
pub mod interaction;
pub mod testing;
pub mod widget_dev;
pub mod widgets;

/// GPU render + scene-compositing surface for embedding host engines.
/// Off unless the `render` feature is enabled. See DR-054.
#[cfg(feature = "render")]
pub mod render;
/// Text-shaping surface for embedding host engines.
/// Off unless the `render` feature is enabled. See DR-054.
#[cfg(feature = "render")]
pub mod text;

/// Host-agnostic UI driver (`UiContext`) for embedding sigil in another
/// engine's frame loop. Off unless the `render` feature is enabled. See DR-054.
#[cfg(feature = "render")]
pub mod ui {
    pub use app::UiContext;
}

/// Reactive primitives re-exported unchanged.
pub mod reactive {
    pub use reactive::{
        EventChannel, Runtime, Signal, Trigger, batch, derived, derived_eq, effect, effect_named,
        on_cleanup, signal,
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
    pub use actions::{Action, DeriveAction, DispatchCx, HandlerKey};
    pub use events::prelude::*;
    pub use kernel::prelude::*;
    pub use layout::prelude::*;
    pub use reactive::{EventChannel, Signal, Trigger, derived, effect, signal};
    /// Utility-class strings accepted by `.class("...")` include:
    /// `p-N`, `px-N`, `py-N`, `pt-N`, `pr-N`, `pb-N`, `pl-N`, `m-N`, `mx-N`,
    /// `my-N`, `mt-N`, `mr-N`, `mb-N`, `ml-N`, `gap-N`, `w-N`, `h-N`,
    /// `min-w-N`, `max-w-N`, `w-full`, `h-full`, `w-auto`, `h-auto`, `flex`,
    /// `block`, `hidden`, `flex-row`, `flex-col`, `flex-row-reverse`,
    /// `flex-col-reverse`, `items-start`, `items-center`, `items-end`,
    /// `items-stretch`, `justify-start`, `justify-center`, `justify-end`,
    /// `justify-between`, `justify-around`, `rounded`, `rounded-sm`,
    /// `rounded-md`, `rounded-lg`, `rounded-full`, `bg-<name>`, `text-<name>`,
    /// `border-<name>`, `shadow-<name>`, `text-xs`, `text-sm`, `text-base`,
    /// `text-lg`, `text-xl`, `font-normal`, `font-medium`, `font-bold`,
    /// `font-sans`, `font-mono`, `font-serif`, `leading-N`, `opacity-N`.
    pub use style::{
        ColorAtom, ColorOrToken, Fill, Flex, FocusRing, Geometry, NodeStyleExt, Rgba8, Sizing,
        Spacing, StateStyle, StateVariants, Style, StyleAccess, StyleAtom, StyleState,
        StyleVariant, Styleable, Styled, Theme, ThemeValue, Tone, Transitions, Typography, Visual,
    };
    // NOTE — `style::Size` (the semantic widget-size enum `Compact /
    // Default / Wide / Grow`) is NOT in the prelude because it
    // collides with `hal::Size::fixed(w, h)` re-exported through
    // `layout::prelude`. Reach for it as `style::Size` at the call
    // site (the widget builders' `.size(...)` methods accept it).
    pub use tasks::{OwnerHandle, dispatch_to_owner};

    pub use crate::interaction::InteractionState;
    pub use crate::reactive::Runtime;
    pub use crate::widgets::layout_ext::WidgetLayoutExt;
    pub use crate::widgets::patterns;
    pub use crate::widgets::{
        ButtonBuilder, CheckboxBuilder, CheckboxTone, ChipTone, Clipboard, CodeEditBuilder,
        CommandMenuBuilder, DotBuilder, DropdownBuilder, FctSpawn, FloaterStyle,
        FloatingTextBuilder, GaugeBuilder, HintOverlayBuilder, HlSpan, IconBuilder, MenuBuilder,
        MenuItem, ModalBuilder, NumberFieldBuilder, NumericKind, Orientation, PanelBuilder,
        ProgressBuilder, RadioGroupBuilder, ScrollAnchor, ScrollBuilder, SelectionMode,
        SelectionState, SliderBuilder, SplitterBuilder, SplitterOrientation, SubmenuBuilder,
        TabsBuilder, TextEditBuilder, TextFieldBuilder, ToggleBuilder, TooltipBuilder,
        TurnStripBuilder, VirtualListBuilder, button, checkbox, chip, code_edit, command_menu,
        divider_horizontal, divider_vertical, dot, dropdown, floating_text, gauge, icon, kv_row,
        label, menu, modal, number_field, panel, progress, radio_group, scroll, slider, splitter,
        status_bar, tabs, text_edit, text_field, toggle, tooltip, turn_strip, virtual_list,
    };
    pub use crate::{AppConfig, AppResult, Error, launch};
}
