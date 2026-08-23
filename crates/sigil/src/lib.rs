//! GPU UI in Rust. One widget tree. Three ways to run it:
//!
//! - [`launch`]: desktop window (feature `desktop`, default)
//! - [`Host`]: you own the loop (game, tests, custom window)
//! - [`run_tui`]: character grid (feature `tui`)
//!
//! Write `use sigil::prelude::*;`. Do not name leaf crates.
//!
//! Style with `.class(...)` (or `classes!` for literals). `.styled()`
//! is the typed method chain. Both merge the same atoms.
//!
//! `signal` / `derived` / `effect` panic outside
//! [`prelude::Runtime::scope`]. [`launch`] and [`run_tui`] open that
//! scope. An embed host that uses signals must call it.

#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

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
use sigil_animation_runtime as _;
#[cfg(test)]
use sigil_bidi as _;
#[cfg(test)]
use sigil_hal as _;
#[cfg(test)]
use sigil_renderer as _;
#[cfg(test)]
use wgpu as _;

/// Native file dialogs and message alerts — see [`sigil_app::dialog`].
#[cfg(feature = "desktop")]
pub use sigil_app::dialog;
/// Native menu bar — see [`sigil_app::menu`].
#[cfg(feature = "desktop")]
pub use sigil_app::menu;
/// Open a URL or path with the OS handler.
#[cfg(feature = "desktop")]
pub use sigil_app::open;
/// System tray icon — see [`sigil_app::tray`].
#[cfg(feature = "desktop")]
pub use sigil_app::tray;
pub use sigil_app::{
    Anchor, AppConfig, AppResult, Error, Host, Painted, PanicPolicy, Player, ResizeDirection,
    Session, TreeRoot, WindowCommand, WindowOpts, audio, pty, root,
};
/// Text engine and GPU tuning. On with `desktop` or `render`.
#[cfg(any(feature = "desktop", feature = "render"))]
pub use sigil_app::{
    DefaultFonts, TextTuning, set_default_fonts, set_text_tuning, text_tuning, use_system_fonts,
};

/// Open a window. Installs the OS clipboard before the tree builds.
///
/// `tree` is a closure, `text(...)`, `image(...)`, a `container()`,
/// or [`root`] wrapping a widget.
///
/// # Errors
///
/// Returns the shell, renderer, or layout error from the app host.
#[cfg(feature = "desktop")]
pub fn launch<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    clipboard::install();
    sigil_app::launch(config, tree)
}

/// Frame profiler — `SIGIL_PROFILE=1` dumps a Perfetto trace + phase table.
pub use sigil_profile as profile;
/// Terminal loop. Same tree and config as [`launch`].
///
/// # Errors
///
/// Returns when the terminal cannot enter raw mode, or when a write fails.
#[cfg(feature = "tui")]
pub fn run_tui<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    clipboard::install();
    sigil_tui::run(config, tree)
}
/// Periodic work on the terminal frame clock, slow work off it, and the
/// buffers a host that runs its own loop keeps between frames.
#[cfg(feature = "tui")]
pub use sigil_tui::{Screen, every, work};

pub mod clipboard;
pub mod input_glyphs;
pub mod testing;
pub use sigil_widgets as widgets;
pub use sigil_widgets::{interaction, widget_dev};

/// GPU scene types for an embed host that submits to wgpu.
///
/// Feature `render`. [`launch`] does not need this module.
#[cfg(feature = "render")]
pub mod render;
/// Text-shaping types for an embed host.
///
/// Feature `render`. [`launch`] does not need this module.
#[cfg(feature = "render")]
pub mod text;

/// Signal graph: [`signal`](crate::reactive::signal), [`derived`](crate::reactive::derived), [`effect`](crate::reactive::effect), [`EventChannel`](crate::reactive::EventChannel).
///
/// Writes go through [`Signal::set`](crate::reactive::Signal::set). Background work is
/// [`crate::prelude::spawn`], which returns a [`Signal`](crate::reactive::Signal).
/// Entity, `EntityCx`, `Task`, and `AsyncContext` stay on
/// `sigil_reactive` for tests. They are not this module.
pub mod reactive {
    pub use sigil_reactive::{
        EventChannel, Runtime, Signal, batch, derived, derived_eq, effect, effect_named,
        on_cleanup, signal,
    };
}

/// Prelude: the imports a user writes at the top of an app.
///
/// Tree builders, layout, style, signals, [`Host`], [`Painted`],
/// [`Player`], [`Session`], [`root`], [`launch`] (desktop), [`open`]
/// (desktop), [`run_tui`] (tui), [`copy`], [`paste`], and
/// [`set_clipboard`].
/// Config types also live at the crate root. Widget helpers are
/// [`IntoNode`](sigil_kernel::IntoNode); type a helper as `impl IntoNode`.
pub mod prelude {
    pub use sigil_actions::{Action, DeriveAction, HandlerKey, Propagate};
    pub use sigil_events::prelude::*;
    pub use sigil_kernel::prelude::*;
    pub use sigil_layout::prelude::*;
    pub use sigil_reactive::{EventChannel, Signal, derived, effect, signal};
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
    /// paint elevation drop shadows. `sm:`/`md:`/`lg:` prefixes are rejected.
    /// `hover:`/`focus:`/`active:`/`disabled:` are rejected on a plain
    /// `.class(...)` (no interaction channel). Use
    /// `interactive(...).class("hover:bg-accent-9")` or a widget
    /// `.hover(|s| …)` closure.
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
    pub use sigil_tasks::{OwnerHandle, Status, dispatch_to_owner, dispatcher, post, spawn};

    pub use crate::clipboard::{copy, paste, set_clipboard};
    pub use crate::interaction::InteractionState;
    #[cfg(feature = "desktop")]
    pub use crate::launch;
    #[cfg(feature = "desktop")]
    pub use crate::open;
    pub use crate::reactive::Runtime;
    pub use crate::root;
    #[cfg(feature = "tui")]
    pub use crate::run_tui;
    #[cfg(feature = "image-decode")]
    pub use crate::widgets::image_bytes;
    pub use crate::widgets::layout_ext::WidgetLayoutExt;
    pub use crate::widgets::patterns;
    pub use crate::widgets::{
        ButtonBuilder, ChartBuilder, ChartMark, CheckboxBuilder, ChipBuilder, ClipError, Clipboard,
        CodeEditBuilder, ColWidth, Combobox, CommandMenuBuilder, DRAG_THRESHOLD_PX,
        DROP_TARGET_CLASS, Date, DateFieldBuilder, Density, DotBuilder, DragDropExt, DragState,
        DropdownBuilder, FctSpawn, FloaterStyle, FloatingTextBuilder, GaugeBuilder,
        HintOverlayBuilder, HlSpan, IconBuilder, Identifiable, ImageBuilder, InteractiveBuilder,
        MenuBarBuilder, MenuBuilder, MenuItem, ModalBuilder, NumberFieldBuilder, NumericKind,
        Orientation, PanelBuilder, PathCmd, PathFill, PathNode, ProgressBuilder, RadioGroupBuilder,
        ScrollAnchor, ScrollBuilder, SelectionMode, SelectionState, Side, SliderBuilder,
        SplitterBuilder, SplitterOrientation, SubmenuBuilder, TableBuilder, TableColumn,
        TableState, TabsBuilder, TextEditBuilder, TextFieldBuilder, ToastEntry, ToastLevel,
        ToastQueue, ToggleBuilder, TooltipBuilder, TransportBuilder, TurnStripBuilder,
        VirtualListBuilder, avatar, axis_drag, badge, button, card, chart, checkbox, chip,
        code_edit, color_field, combobox, command_menu, date_field, divider_horizontal,
        divider_vertical, dot, drag_ghost, dropdown, field, floating_text, gauge, html, icon,
        icon_button, image_rgba, interactive, kv_row, label, markdown, markdown_edit, menu,
        menu_bar, mermaid, modal, number_field, panel, path, progress, radio_group, scroll, sheet,
        sidebar, sidebar_width, slider, splitter, status_bar, surface, table, tabs, terminal,
        text_edit, text_field, toast, toast_queue, toggle, tooltip, transport, turn_strip,
        validated_field, virtual_list,
    };
    pub use crate::{AppConfig, AppResult, Error, Host, Painted, Player, Session, TreeRoot};
}
