#![cfg_attr(
    all(
        feature = "scene",
        not(any(test, feature = "desktop", feature = "kittest"))
    ),
    allow(
        dead_code,
        reason = "a build without a window uses only the CPU capture path"
    )
)]
#![doc = include_str!("../docs/overview.md")]
#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]
#![cfg_attr(
    not(feature = "kittest"),
    allow(unreachable_pub, reason = "items hidden behind kittest stay pub")
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
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) mod a11y;
/// Tweens, springs, and animators.
pub mod animation;
/// Windows, the frame loop, dialogs, menus, and other platform services.
pub mod app;
pub(crate) mod bidi;
mod error;
/// Input events, shortcuts, and focus requests.
pub mod events;
mod fnv;
/// Points, sizes, and rectangles.
pub mod geometry;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub mod hal;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) mod hal;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) mod hud;
/// The icon catalog and icon baking.
#[cfg(feature = "scene")]
pub mod icons;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub mod kernel;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) mod kernel;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub mod layout;
#[cfg(not(feature = "kittest"))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) mod layout;
/// Frame timing, counters, and profile output.
pub mod profile;
/// Signals, effects, owners, and channels.
pub mod reactive;
#[cfg(all(feature = "scene", feature = "kittest"))]
#[doc(hidden)]
pub mod renderer;
#[cfg(all(feature = "scene", not(feature = "kittest")))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) mod renderer;
/// The drawing commands a frame produces.
pub mod scene;
#[cfg(all(feature = "desktop", feature = "kittest"))]
#[doc(hidden)]
pub mod shell;
#[cfg(all(feature = "desktop", not(feature = "kittest")))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) mod shell;
/// Tokens, themes, colors, and style values.
pub mod style;
/// Async tasks and writes to signals from other threads.
pub mod tasks;
/// The terminal host.
#[cfg(feature = "terminal")]
pub mod terminal;
#[cfg(all(feature = "scene", feature = "kittest"))]
#[doc(hidden)]
pub mod text;
#[cfg(all(feature = "scene", not(feature = "kittest")))]
#[allow(unused_imports, reason = "public only under kittest")]
pub(crate) mod text;
/// The widget catalog.
mod view;
pub mod widgets;

pub use crate::app::config::AppConfig;
pub use crate::app::embed::Host;
pub use crate::app::exit::request_exit;
pub use crate::error::{AppResult, Error, ErrorKind};
pub use crate::kernel::{Cx, Live, Node, NodeId, Surface, TopLayer, announce};
pub use crate::view::View;

/// Run the app component on the host that `config` selects.
#[cfg(any(feature = "desktop", feature = "terminal"))]
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
#[cfg(any(test, feature = "kittest"))]
pub mod testing;

#[cfg(fuzzing)]
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
    #[cfg(feature = "desktop")]
    pub use crate::app::open;
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub use crate::app::{ThemeWatcher, watch_theme};
    pub use crate::events::request_focus;
    pub use crate::events::{
        Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState, MouseButton, MouseButtons,
        PointerEvent, PointerPhase, PointerType, Shortcut,
    };
    pub use crate::geometry::{Point, Size};
    #[cfg(feature = "scene")]
    pub use crate::icons::Icon;
    pub use crate::kernel::{
        Action, HostMode, ImageBuilder, ImageSource, IntoNode, NodeBuilder, ObjectFit, Role,
        TextBuilder, TextContent, TextCursor, container, for_each, fragment, image, node, show,
        switch, text,
    };
    pub use crate::layout::audit::Flaw;
    kittest_pub! {
        #[doc(hidden)]
        pub use crate::layout::LayoutStyle;
    }
    kittest_pub! {
        #[doc(hidden)]
        pub use crate::widgets::Layout;
    }
    pub use crate::layout::{
        AlignItems, FlexWrap, JustifyContent, Length, Overflow, Position, Space, TrackSize, cols,
        column, em, grid, row, rows,
    };
    pub use crate::reactive::{IntervalId, Reactive, Signal, derived, effect, interval, signal};
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub use crate::run;
    pub use crate::style::{
        Color, ControlSize, Elevation, FontFamily, Layer, Paint, Radius, States, TextRole, Theme,
        Token, Tone,
    };
    pub use crate::tasks::{SignalWriter, Status, Task, signal_writer, spawn, task};
    pub use crate::view::View;
    #[cfg(feature = "a2ui")]
    pub use crate::widgets::a2ui;
    pub use crate::widgets::{
        ColumnWidth, Combobox, Date, Density, DiffView, Disclosure, DragState, Identifiable,
        ListRowSlots, MenuItem, ModalBuilder, Orientation, PathNode, SelectionMode, Side,
        SortDirection, SortKey, TableColumn, TableState, ToastQueue, Widget, app_shell, avatar,
        axis_drag, button, card, chart, checkbox, chip, code_edit, color_field, combobox,
        command_menu, context_menu, date_field, diff, divider, dot, drag_ghost, dropdown,
        empty_state, field, fuzzy_score, gauge, graphviz, heatmap, highlight, hint_bar,
        hint_overlay, html, icon, icon_button, kv_grid, kv_row, list_row, markdown, markdown_edit,
        markdown_inline, math, menu, menu_bar, mermaid, modal, notice_band, number_field, panel,
        path, progress, radio_group, scroll, section_label, segmented, sheet, sidebar, slider,
        spacer, sparkline, spinner, splitter, stat, status_bar, surface, table, tabs, terminal,
        text_edit, text_field, toast, toast_queue, toggle, tooltip, tree, virtual_list,
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
