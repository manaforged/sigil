#![deny(missing_docs)]
#![cfg_attr(
    all(feature = "scene", not(window_host)),
    allow(
        dead_code,
        reason = "a build without a window uses only the CPU capture path"
    )
)]
#![doc = include_str!("../docs/overview.md")]
#![allow(
    clippy::wildcard_imports,
    reason = "a module split into files imports its parent with `use super::*`"
)]

extern crate alloc;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use image as _;
#[cfg(test)]
use pollster as _;
#[cfg(all(test, not(feature = "scene")))]
use wgpu as _;

macro_rules! kittest_pub {
    (if $cond:meta; $(#[$meta:meta])* pub $($item:tt)*) => {
        $(#[$meta])*
        #[cfg(feature = "kittest")]
        pub $($item)*
        $(#[$meta])*
        #[cfg(all(not(feature = "kittest"), $cond))]
        pub(crate) $($item)*
    };
    ($(#[$meta:meta])* pub $($item:tt)*) => {
        $(#[$meta])*
        #[cfg(feature = "kittest")]
        pub $($item)*
        $(#[$meta])*
        #[cfg(not(feature = "kittest"))]
        #[allow(unused_imports, reason = "public only under kittest")]
        pub(crate) $($item)*
    };
}

#[cfg(feature = "kittest")]
#[doc(hidden)]
pub mod a11y;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, unreachable_pub, reason = "public only under kittest")]
pub(crate) mod a11y;
/// Tweens, springs, and animators.
pub mod animation;
/// Windows, the frame loop, dialogs, menus, and other platform services.
pub mod app;
pub(crate) mod bidi;
mod cache;
mod error;
/// Input events, shortcuts, and focus requests.
pub mod events;
mod fnv;
/// Points, sizes, and rectangles.
pub mod geometry;
mod grapheme;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub mod hal;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, unreachable_pub, reason = "public only under kittest")]
pub(crate) mod hal;
#[cfg(app_host)]
pub(crate) mod hud;
/// The icon catalog and icon baking.
#[cfg(feature = "scene")]
pub mod icons;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub mod kernel;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, unreachable_pub, reason = "public only under kittest")]
pub(crate) mod kernel;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub mod layout;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, unreachable_pub, reason = "public only under kittest")]
pub(crate) mod layout;
#[cfg(http_fetch)]
mod net;
/// Frame timing, counters, and profile output.
pub mod profile;
/// Signals, effects, owners, and channels.
pub mod reactive;
#[cfg(scene_harness)]
#[doc(hidden)]
pub mod renderer;
#[cfg(scene_internal)]
#[allow(unused_imports, unreachable_pub, reason = "public only under kittest")]
pub(crate) mod renderer;
/// The drawing commands a frame produces.
pub mod scene;
#[cfg(desktop_harness)]
#[doc(hidden)]
pub mod shell;
#[cfg(live_desktop)]
#[allow(unused_imports, unreachable_pub, reason = "public only under kittest")]
pub(crate) mod shell;
/// Tokens, themes, colors, and style values.
pub mod style;
/// Async tasks and writes to signals from other threads.
pub mod tasks;
/// The terminal host.
#[cfg(feature = "terminal")]
pub mod terminal;
#[cfg(scene_harness)]
#[doc(hidden)]
pub mod text;
#[cfg(scene_internal)]
#[allow(unused_imports, unreachable_pub, reason = "public only under kittest")]
pub(crate) mod text;
mod view;
/// The widget catalog.
pub mod widgets;

pub use crate::app::config::AppConfig;
pub use crate::app::embed::Host;
pub use crate::app::exit::exit;
pub use crate::error::{AppResult, Error, ErrorKind};
pub use crate::kernel::{Cx, Live, Node, NodeId, Surface, TopLayer, announce};
pub use crate::view::View;

/// Run the app component on the host that `config` selects.
#[cfg(app_host)]
pub fn run<C, F, N>(config: C, app: F) -> AppResult
where
    C: Into<AppConfig>,
    F: Fn() -> N + 'static,
    N: crate::kernel::IntoNode,
{
    let config = config.into();
    app::platform_prefs::apply(&config);
    match config.host {
        #[cfg(feature = "terminal")]
        kernel::HostMode::Cell => terminal::run(&config, app),
        #[cfg(feature = "desktop")]
        kernel::HostMode::Pixel => {
            clipboard::install_os();
            app::launch(config, app)
        }
        #[allow(
            unreachable_patterns,
            reason = "the arm exists when a host feature is off"
        )]
        _ => Err(Error::config(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "sigil-ui was built without the feature for this host",
        ))),
    }
}

/// Read and write the system clipboard.
pub mod clipboard;
/// Helpers that Sigil's own tests use to reach the kernel.
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub mod testing;

#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzzing {
    pub fn render_mermaid(input: &str) {
        drop(crate::widgets::mermaid_figure(input));
    }

    #[must_use]
    pub fn parse_class_string(input: &str) -> bool {
        crate::style::parse_class_string(input).is_ok()
    }

    #[must_use]
    pub fn parse_stateful_token(input: &str) -> bool {
        crate::style::parse_stateful_token(input).is_ok()
    }

    #[cfg(all(feature = "terminal", unix))]
    pub fn probe_reply(input: &str) {
        crate::terminal::fuzz_reply(input);
    }

    #[cfg(feature = "terminal")]
    pub fn reply_filter(input: &[u8]) {
        crate::terminal::fuzz_strip(input);
    }
}

/// The items most apps use.
pub mod prelude {
    pub use crate::animation::{Easing, Keyframes, Property, Transition};
    #[cfg(app_host)]
    pub use crate::app::open_url;
    #[cfg(app_host)]
    pub use crate::app::{ThemeWatcher, watch_theme};
    pub use crate::events::{
        Key, KeyEvent, KeyPhase, Modifiers, MouseButton, MouseButtons, NamedKey, PointerEvent,
        PointerPhase, PointerType, Shortcut,
    };
    pub use crate::geometry::{Constraint, Point, Rect, Size};
    #[cfg(feature = "scene")]
    pub use crate::icons::Icon;
    #[doc(hidden)]
    pub use crate::kernel::NodePatch;
    pub use crate::kernel::{
        AccessibleName, Action, HostMode, ImageBuilder, ImageSource, IntoNode, NodeBuilder,
        ObjectFit, Role, SwitchBuilder, TextBuilder, TextContent, container, for_each, fragment,
        image, node, show, switch, text,
    };
    pub use crate::layout::audit::Flaw;
    #[cfg(feature = "image-decode")]
    pub use crate::widgets::{ImageLoadBuilder, load_image};
    kittest_pub! {
        #[doc(hidden)]
        pub use crate::layout::LayoutStyle;
    }
    kittest_pub! {
        #[doc(hidden)]
        pub use crate::widgets::Layout;
    }
    pub use crate::layout::{
        AlignItems, Display, FlexWrap, JustifyContent, Length, Overflow, Position, Space,
        TrackSize, cols, column, em, grid, row, rows,
    };
    pub use crate::reactive::{Reactive, Signal, Timer, effect, interval, memo, signal};
    #[cfg(app_host)]
    pub use crate::run;
    pub use crate::scene::SceneBuilder;
    pub use crate::style::{
        Background, Color, ControlSize, Elevation, FocusRing, FontFamily, Gradient, Layer, Paint,
        Radius, States, TextRole, Theme, Token, Tone,
    };
    pub use crate::tasks::{SignalWriter, Status, Task, spawn, task};
    pub use crate::view::View;
    #[cfg(feature = "a2ui")]
    pub use crate::widgets::a2ui;
    pub use crate::widgets::{
        Change, ColumnAlign, ColumnWidth, Combobox, Date, DateTime, Density, Diagnostic,
        DiffSelection, DiffView, Disclosure, DragState, FuzzyMatch, Glyph, Highlight, Identifiable,
        ListRowBuilder, Log, MenuItem, ModalBuilder, NumericKind, Orientation, Patch, PathBuilder,
        RowTone, ScrollAnchor, Selection, SelectionMode, Severity, Side, SortDirection, SortKey,
        SparkFill, SparkMark, SubmitKey, TableColumn, TableState, TextFieldBuilder, TextState,
        Time, TimeValue, ToastQueue, TreeNode, Widget, app_shell, avatar, axis_drag, badge,
        breadcrumbs, button, canvas, card, chart, checkbox, chip, code_edit, color_field, combobox,
        command_menu, context_menu, date_field, diff, disclosure, divider, dot, drag_ghost,
        dropdown, empty_state, field, fuzzy_score, gauge, graphviz, heatmap, highlight, hint_bar,
        hint_overlay, html, icon, kv_grid, kv_row, link, list_row, log_view, markdown,
        markdown_edit, markdown_inline, math, menu, menu_bar, mermaid, modal, notice_band,
        number_field, pagination, path, progress, radio_group, range_slider, scroll, section_label,
        segmented, sidebar, skeleton, slider, spacer, sparkline, spinner, splitter, stat,
        status_bar, stepper, surface, table, tabs, tag_input, terminal, text_field, time_field,
        toast, toast_queue, toggle, tooltip, tree, vega, virtual_list,
    };
    pub use crate::{AppConfig, AppResult, Error};
}

#[cfg(doctest)]
mod doc_snippets {
    #[doc = include_str!("../../../README.md")]
    pub struct Readme;
    #[doc = include_str!("../README.md")]
    pub struct PackageReadme;
    #[doc = include_str!("../../../docs/guide/README.md")]
    pub struct GuideReadme;
    #[doc = include_str!("../../../docs/guide/arch/modules.md")]
    pub struct GuideArchModules;
    #[doc = include_str!("../../../docs/guide/arch/overview.md")]
    pub struct GuideArchOverview;
    #[doc = include_str!("../../../docs/guide/arch/painting.md")]
    pub struct GuideArchPainting;
    #[doc = include_str!("../../../docs/guide/arch/platform.md")]
    pub struct GuideArchPlatform;
    #[doc = include_str!("../../../docs/guide/arch/state.md")]
    pub struct GuideArchState;
    #[doc = include_str!("../../../docs/guide/decisions/DR-001-signals-over-entity.md")]
    pub struct GuideDecisionsDr001SignalsOverEntity;
    #[doc = include_str!("../../../docs/guide/decisions/README.md")]
    pub struct GuideDecisionsReadme;
    #[cfg_attr(feature = "desktop", doc = include_str!("../../../docs/guide/hot-reload.md"))]
    pub struct GuideHotReload;
    #[doc = include_str!("../../../docs/guide/packing.md")]
    pub struct GuidePacking;
    #[doc = include_str!("../../../docs/guide/quickstart.md")]
    pub struct GuideQuickstart;
    #[doc = include_str!("../../../docs/guide/recipes.md")]
    pub struct GuideRecipes;
    #[cfg_attr(feature = "terminal", doc = include_str!("../../../docs/guide/terminal.md"))]
    pub struct GuideTerminal;
    #[doc = include_str!("../../../docs/guide/troubleshooting.md")]
    pub struct GuideTroubleshooting;
    #[doc = include_str!("../../../docs/guide/errors.md")]
    pub struct GuideErrors;
    #[cfg_attr(feature = "terminal", doc = include_str!("../../../docs/guide/hosts.md"))]
    pub struct GuideHosts;
    #[doc = include_str!("../../../docs/guide/units.md")]
    pub struct GuideUnits;
    #[doc = include_str!("../../../docs/guide/web.md")]
    pub struct GuideWeb;
    #[doc = include_str!("../../../docs/guide/widgets/chrome.md")]
    pub struct GuideWidgetsChrome;
    #[doc = include_str!("../../../docs/guide/widgets/content.md")]
    pub struct GuideWidgetsContent;
    #[doc = include_str!("../../../docs/guide/widgets/controls.md")]
    pub struct GuideWidgetsControls;
    #[doc = include_str!("../../../docs/guide/widgets/data.md")]
    pub struct GuideWidgetsData;
    #[doc = include_str!("../../../docs/guide/widgets/documents.md")]
    pub struct GuideWidgetsDocuments;
    #[doc = include_str!("../../../docs/guide/widgets/fields.md")]
    pub struct GuideWidgetsFields;
    #[doc = include_str!("../../../docs/guide/widgets/indicators.md")]
    pub struct GuideWidgetsIndicators;
    #[doc = include_str!("../../../docs/guide/widgets/overlays.md")]
    pub struct GuideWidgetsOverlays;
    #[doc = include_str!("../../../docs/guide/widgets/plumbing.md")]
    pub struct GuideWidgetsPlumbing;
    #[doc = include_str!("../../../docs/guide/widgets.md")]
    pub struct GuideWidgets;
}
