#![doc = include_str!("../README.md")]
#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

extern crate alloc;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use image as _;
#[cfg(test)]
use pollster as _;
#[cfg(all(test, not(feature = "scene")))]
use wgpu as _;
#[cfg(feature = "scene")]
pub use wgpu;

pub mod a11y;
pub mod animation;
pub mod animation_runtime;
pub mod app;
pub mod bidi;
pub mod events;
pub mod geometry;
pub mod hal;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) mod hud;
#[cfg(feature = "scene")]
pub mod icons;
pub mod kernel;
pub mod layout;
pub mod profile;
pub mod reactive;
#[cfg(feature = "scene")]
pub mod renderer;
pub mod scene;
#[cfg(feature = "shell")]
pub mod shell;
pub mod style;
pub mod tasks;
#[cfg(feature = "terminal")]
pub mod terminal;
#[cfg(feature = "scene")]
pub mod text;
pub mod widgets;

pub use crate::app::config::AppConfig;
pub use crate::app::embed::Host;
pub use crate::app::error::{AppResult, Error};
pub use crate::app::exit::request_exit;

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
}

pub mod prelude {
    #[cfg(feature = "desktop")]
    pub use crate::app::open;
    pub use crate::app::set_theme;
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub use crate::app::{ThemeWatcher, watch_theme};
    pub use crate::events::{
        Key, KeyEvent, KeyPhase, LogicalKey, ModifiersState, MouseButton, MouseButtons,
        PointerEvent, PointerKind, PointerPhase, Shortcut,
    };
    pub use crate::events::{NodeEventsExt, request_focus};
    pub use crate::geometry::{Point, Size};
    pub use crate::kernel::{
        Action, ImageSource, IntoNode, NodeBuilder, Role, TextContent, container, for_each,
        fragment, image, match_status, node, show, switch, text,
    };
    pub use crate::layout::{
        AlignItems, Display, FlexWrap, IntoPx, JustifyContent, LayoutBuilderExt, LayoutReactiveExt,
        LayoutStyle, cols, column, em, row, rows,
    };
    pub use crate::reactive::{IntervalId, Reactive, Signal, derived, effect, interval, signal};
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub use crate::run;
    pub use crate::style::{
        ColorAtom, ControlSize, Elevation, FocusRing, FontFamily, NodeStyleExt, Paint, Radius,
        Rgba8, StateVariants, Theme, Token, Tone,
    };
    pub use crate::tasks::{
        ScheduleError, SignalWriter, Status, Task, signal_writer, spawn, task, task_with,
    };
    #[cfg(feature = "scene")]
    pub use crate::widgets::icon_svg;
    #[cfg(feature = "image-decode")]
    pub use crate::widgets::image_bytes;
    pub use crate::widgets::layout_ext::WidgetLayoutExt;
    pub use crate::widgets::{
        ColWidth, Combobox, Date, Density, DiffView, Disclosure, DragDropExt, DragState,
        Identifiable, ListRowSlots, MenuItem, ModalBuilder, NumericKind, Orientation, PathNode,
        SelectionMode, Side, SortDir, SortKey, SubmitKey, TableColumn, TableState, ToastQueue,
        app_shell, avatar, axis_drag, button, card, chart, checkbox, chip, code_edit, color_field,
        combobox, command_menu, context_menu, date_field, diff, divider_horizontal,
        divider_vertical, dot, drag_ghost, dropdown, empty_state, field, fuzzy_score, gauge,
        hairline, heatmap, highlight, hint_bar, hint_overlay, hl_spans, html, icon, icon_button,
        image_rgba, interactive, kv_grid, kv_row, list_row, list_row_with, markdown, markdown_edit,
        markdown_inline, menu, menu_bar, mermaid, modal, notice_band, number_field, panel, path,
        progress, radio_group, scroll, section_label, segmented, sheet, sidebar, slider, spacer,
        sparkline, spinner, spinner_frame, splitter, status_bar, surface, table, tabs, terminal,
        text_edit, text_field, toast, toast_queue, toggle, tooltip, virtual_list,
    };
    pub use crate::{AppConfig, AppResult};
}

#[cfg(doctest)]
mod doc_snippets {
    #[doc = include_str!("../../../README.md")]
    pub struct Readme;
    #[doc = include_str!("../../../docs/api.md")]
    pub struct Api;
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
    #[doc = include_str!("../../../docs/guide/decisions/DR-058-signals-over-entity.md")]
    pub struct GuideDecisionsDr058SignalsOverEntity;
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
