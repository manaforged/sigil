//! The sigil UI framework.
//!
//! Thin facade over [`sigil_app`] and the kernel crates so user code
//! writes `use sigil::prelude::*;` and never has to name individual
//! sub-crates. See DR-016 (app launch) and the ergonomics-pass plan.
//!
//! ```rust,no_run
//! use sigil::prelude::*;
//!
//! fn main() -> sigil::AppResult<()> {
//!     sigil::launch(("Demo", 480, 240), app_tree)
//! }
//!
//! fn app_tree(cx: &mut Cx) -> NodeId {
//!     cx.add(container().class("bg-accent"))
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
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
extern crate self as sigil;

/// Macro-expansion support — NOT public API.
///
/// `classes!` (sigil-style-macros) and `#[derive(Action)]`
/// (sigil-actions-macros) emit absolute paths through this module so
/// expansions resolve in any crate that depends on the facade alone —
/// preserving the prelude-only rule. Everything here is `#[doc(hidden)]`
/// and may change between lockstep releases.
#[doc(hidden)]
pub mod __private {
    pub use ::sigil_actions as actions;
    pub use ::sigil_style as style;
}

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

pub use sigil_app::{Anchor, WindowOpts};
pub use sigil_app::{
    AppConfig, AppResult, DefaultFonts, Error, PanicPolicy, TextTuning, TreeRoot, launch,
    set_default_fonts, set_text_tuning, text_tuning,
};

pub mod clipboard;
pub mod input_glyphs;
pub mod interaction;
pub mod testing;
pub mod widget_dev;
pub mod widgets;

/// GPU render + scene-compositing surface for embedding host engines.
///
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
    pub use sigil_app::UiContext;
}

/// Reactive primitives re-exported unchanged.
pub mod reactive {
    pub use sigil_reactive::{
        AsyncContext, Entity, EventChannel, Runtime, Signal, Task, Trigger, batch, derived,
        derived_eq, effect, effect_named, on_cleanup, signal,
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
    pub use sigil_events::prelude::*;
    pub use sigil_kernel::prelude::*;
    pub use sigil_layout::prelude::*;
    pub use sigil_reactive::{EventChannel, Signal, Trigger, derived, effect, signal};
    /// Utility-class strings accepted by `.class("...")` (and `classes!`,
    /// which validates them at compile time — the default for literals):
    /// `p-N`, `px-N`, `py-N`, `pt-N`, `pr-N`, `pb-N`, `pl-N`, `m-N`, `mx-N`,
    /// `my-N`, `mt-N`, `mr-N`, `mb-N`, `ml-N`, `gap-N`, `w-N`, `h-N`,
    /// `min-w-N`, `max-w-N`, `min-h-N`, `max-h-N`, `w-full`, `h-full`,
    /// `w-auto`, `h-auto`, `flex`, `block`, `hidden`, `flex-row`,
    /// `flex-col`, `flex-row-reverse`, `flex-col-reverse`, `grow`, `shrink`,
    /// `grow-0`, `shrink-0`, `items-start`, `items-center`, `items-end`,
    /// `items-stretch`, `justify-start`, `justify-center`, `justify-end`,
    /// `justify-between`, `justify-around`, `overflow-visible`,
    /// `overflow-hidden`, `overflow-scroll`, `overflow-x-*`, `overflow-y-*`,
    /// `rounded`, `rounded-sm`, `rounded-md`, `rounded-lg`, `rounded-full`,
    /// `border`, `border-2`, `bg-<name>`, `text-<name>`, `border-<name>`,
    /// `text-xs`, `text-sm`, `text-base`, `text-lg`, `text-xl`,
    /// `font-normal`, `font-medium`, `font-bold`, `font-sans`, `font-mono`,
    /// `font-serif`, `font-icon`, `leading-N`, `opacity-N`.
    ///
    /// Scale conventions (T4, documented once and for all): spacing/size
    /// tokens are Tailwind-style 4px steps (`p-4` = 16px, `w-4` = 16px) —
    /// the fluent (`.px(16)`) and struct (`Style::padding(16)`) APIs take
    /// raw logical px instead. `opacity-N` is a percent (`opacity-50` =
    /// 50%) while `.opacity(0.5)` everywhere takes the CSS-canonical
    /// fraction. `shadow-sm`/`shadow`/`shadow-md`/`shadow-lg`/`shadow-xl`
    /// paint elevation drop shadows; the `sm:`/`hover:` prefixes are still
    /// rejected loudly rather than silently ignored.
    ///
    /// Prefer `classes!("p-4 bg-accent")` for string literals: it validates
    /// tokens at compile time (with nearest-match suggestions) instead of
    /// warning at runtime.
    pub use sigil_style::{
        ColorAtom, ColorOrToken, Fill, Flex, FocusRing, Geometry, NodeStyleExt, Rgba8, Sizing,
        Spacing, StateStyle, StateVariants, Style, StyleAccess, StyleAtom, StyleState,
        StyleVariant, Styleable, Styled, Theme, ThemeValue, Tone, Transitions, Typography, Visual,
        classes,
    };
    // NOTE — `sigil_style::Size` (the semantic widget-size enum `Compact /
    // Default / Wide / Grow`) is NOT in the prelude because it
    // collides with `sigil_hal::Size::fixed(w, h)` re-exported through
    // `sigil_layout::prelude`. Reach for it as `sigil_style::Size` at the call
    // site (the widget builders' `.size(...)` methods accept it).
    pub use sigil_tasks::{OwnerHandle, dispatch_to_owner};

    pub use crate::interaction::InteractionState;
    pub use crate::reactive::Runtime;
    #[cfg(feature = "image-decode")]
    pub use crate::widgets::image_bytes;
    pub use crate::widgets::layout_ext::WidgetLayoutExt;
    pub use crate::widgets::patterns;
    pub use crate::widgets::{
        ButtonBuilder, CheckboxBuilder, CheckboxTone, ChipBuilder, ChipTone, Clipboard,
        CodeEditBuilder, ColWidth, CommandMenuBuilder, DotBuilder, DropdownBuilder, FctSpawn,
        FloaterStyle, FloatingTextBuilder, GaugeBuilder, HintOverlayBuilder, HlSpan, IconBuilder,
        ImageBuilder, InteractiveBuilder, MenuBuilder, MenuItem, ModalBuilder, NumberFieldBuilder,
        NumericKind, Orientation, PanelBuilder, ProgressBuilder, RadioGroupBuilder, ScrollAnchor,
        ScrollBuilder, SelectionMode, SelectionState, SliderBuilder, SplitterBuilder,
        SplitterOrientation, SubmenuBuilder, TableBuilder, TableColumn, TableState, TabsBuilder,
        TextEditBuilder, TextFieldBuilder, ToggleBuilder, TooltipBuilder, TurnStripBuilder,
        VirtualListBuilder, axis_drag, button, checkbox, chip, code_edit, command_menu,
        divider_horizontal, divider_vertical, dot, dropdown, floating_text, gauge, icon,
        image_rgba, interactive, kv_row, label, menu, modal, number_field, panel, progress,
        radio_group, scroll, slider, splitter, status_bar, table, tabs, text_edit, text_field,
        toggle, tooltip, turn_strip, virtual_list,
    };
    pub use crate::{AppConfig, AppResult, Error, launch};
}
