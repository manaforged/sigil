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
pub mod hal;
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

#[cfg(feature = "audio")]
pub use crate::app::Player;
#[cfg(feature = "pty")]
pub use crate::app::Session;
#[cfg(feature = "dialog")]
pub use crate::app::dialog;
#[cfg(feature = "menu")]
pub use crate::app::menu;
#[cfg(feature = "desktop")]
pub use crate::app::open;
pub use crate::app::set_theme;
#[cfg(feature = "tray")]
pub use crate::app::tray;
pub use crate::app::{
    AccessibilityTree, Anchor, AppConfig, AppResult, Error, ErrorKind, Host, HostCaps, PanicPolicy,
    ResizeDirection, TreeRoot, WindowCommand, WindowOpts, is_grid, request_exit, root,
    viewport_size,
};
#[cfg(any(feature = "desktop", feature = "render"))]
pub use crate::app::{
    DefaultFonts, TextTuning, set_default_fonts, set_text_tuning, text_tuning, use_system_fonts,
};
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub use crate::app::{ThemeWatcher, parse_theme, watch_theme};

#[cfg(any(feature = "desktop", feature = "terminal"))]
pub fn run<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    let config = config.into();
    match config.host {
        #[cfg(feature = "terminal")]
        kernel::HostMode::Cell => terminal::run(config, tree),
        #[cfg(feature = "desktop")]
        kernel::HostMode::Pixel => {
            clipboard::install_os();
            app::launch(config, tree)
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
pub use crate::widgets::interaction;

#[cfg(feature = "render")]
pub mod render;

pub mod prelude {
    pub use crate::animation::{
        AnimationDirection, AnimationFill, CubicBezier, Easing, Keyframe, Keyframes, SpringPreset,
    };
    pub use crate::animation_runtime::{AnimationConfig, Animator, MotionPrefs, Playback};
    pub use crate::events::{
        EventHandlers, EventRouter, FocusIntent, NodeEventsExt, request_focus, request_reveal,
        request_window_command,
    };
    pub use crate::hal::{
        Dim, FileDropEvent, FileDropPhase, ImeEvent, Key, KeyEvent, KeyPhase, LogicalKey,
        LogicalSize, ModifiersState, MouseButton, MouseButtons, PhysicalKey, Point, PointerEvent,
        PointerEventInit, PointerKind, PointerPhase, ResizeDirection, ScrollDelta, ScrollEvent,
        ScrollPhase, ShellEvent, Shortcut, Size, WindowCommand, WindowEvent,
    };
    pub use crate::kernel::{
        A11yProps, AccessibleName, Action, Cx, ImageCrop, ImageSource, IntoNode, Node, NodeBuilder,
        NodeExtensions, NodeId, NodeType, ObjectFit, Role, Surface, TextBuilder, TextContent,
        TopLayer, container, fragment, image, node, text, text_live,
    };
    pub use crate::layout::{
        BoundsReactive, Dimension, FlexWrap, IntoPx, LayoutBuilderExt, LayoutEngine, LayoutError,
        LayoutReactiveExt, LayoutResult, LayoutStyle, Overflow, ReactivePosition,
        StyleAlignItems as AlignItems, StyleAlignSelf as AlignSelf, StyleDisplay as Display,
        StyleFlexDirection as FlexDirection, StyleJustifyContent as JustifyContent,
        StyleLengthPercentage as LengthPercentage,
        StyleLengthPercentageAuto as LengthPercentageAuto, StylePosition as Position, TextLength,
        TextMeasure, TrackSize, cols, em, rows,
    };
    pub use crate::reactive::{
        EventChannel, IntervalId, Reactive, Signal, batch, defer, derived, derived_eq, effect,
        interval, on_cleanup, signal, untrack,
    };
    pub use crate::style::{
        ColorAtom, ColorOrToken, Dimension as StyleDimension, Fill, Flex, FocusRing, Geometry,
        NodeStyleExt, NumericAtomKind, Px, Rgba8, Sizing, Spacing, StateStyle, StateVariants,
        Style, StyleAccess, StyleAtom, StyleState, StyleVariant, Styleable, Styled, Theme,
        ThemeValue, Tone, Transitions, Typography, Visual, classes,
    };
    pub use crate::tasks::{
        OwnerHandle, ScheduleError, SignalWriter, Status, dispatch_to_owner, dispatcher,
        on_message, on_message_or_close, post, signal_writer, spawn,
    };

    #[cfg(feature = "audio")]
    pub use crate::Player;
    #[cfg(feature = "pty")]
    pub use crate::Session;
    #[cfg(feature = "clipboard")]
    pub use crate::clipboard::OsClipboard;
    pub use crate::clipboard::{ClipError, Clipboard, Delivery, copy, install, paste};
    pub use crate::interaction::InteractionState;
    #[cfg(feature = "desktop")]
    pub use crate::open;
    pub use crate::reactive::Runtime;
    pub use crate::root;
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub use crate::run;
    pub use crate::set_theme;
    #[cfg(feature = "scene")]
    pub use crate::widgets::icon_svg;
    #[cfg(feature = "image-decode")]
    pub use crate::widgets::image_bytes;
    pub use crate::widgets::layout_ext::WidgetLayoutExt;
    pub use crate::widgets::patterns;
    pub use crate::widgets::{
        ButtonBuilder, ChartBuilder, ChartMark, CheckboxBuilder, ChipBuilder, CodeEditBuilder,
        ColAlign, ColWidth, ColorFieldBuilder, Combobox, CommandMenuBuilder, DRAG_THRESHOLD_PX,
        DROP_TARGET_CLASS, Date, DateFieldBuilder, Density, DiffBuilder, DiffLayout, Disclosure,
        DotBuilder, DragDropExt, DragState, DropdownBuilder, FocusCaret, FuzzyMatch, GaugeBuilder,
        Highlight, HintOverlayBuilder, HlSpan, IconBuilder, Identifiable, ImageBuilder,
        InteractiveBuilder, ListColumn, ListRowSlots, MarkdownEditBuilder, MenuBarBuilder,
        MenuBuilder, MenuItem, ModalBuilder, NumberFieldBuilder, NumericKind, Orientation,
        PanelBuilder, PathCmd, PathColor, PathFill, PathNode, ProgressBuilder, RadioGroupBuilder,
        RowTone, ScrollAnchor, ScrollBuilder, SegmentedBuilder, SelectionMode, SelectionState,
        Side, SliderBuilder, SplitterBuilder, SplitterOrientation, SubmenuBuilder, TableBuilder,
        TableColumn, TableState, TabsBuilder, TextEditBuilder, TextEditState, TextFieldBuilder,
        ToastEntry, ToastLevel, ToastQueue, ToggleBuilder, TooltipBuilder, VirtualListBuilder,
        app_shell, avatar, axis_drag, badge, button, card, chart, checkbox, chip, code_edit,
        color_field, combobox, command_menu, context_menu, date_field, diff, divider_horizontal,
        divider_vertical, dot, drag_ghost, dropdown, empty_state, field, fuzzy_score, gauge,
        hairline, heatmap, highlight, hint_bar, hint_overlay, hl_spans, html, icon, icon_button,
        image_rgba, interactive, kv_grid, kv_row, label, list_row, list_row_with, markdown,
        markdown_edit, markdown_inline, menu, menu_bar, mermaid, modal, notice_band, number_field,
        panel, path, progress, radio_group, scroll, scroll_auto, section_label, segmented, sheet,
        sidebar, sidebar_width, slider, spacer, spinner, spinner_frame, splitter, status_bar,
        surface, table, tabs, terminal, text_edit, text_field, toast, toast_queue, toggle, tooltip,
        validated_field, virtual_list,
    };
    pub use crate::{
        AppConfig, AppResult, Error, Host, HostCaps, TreeRoot, is_grid, viewport_size,
    };
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub use crate::{ThemeWatcher, parse_theme, watch_theme};
}
