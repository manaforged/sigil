#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]
#![doc = include_str!("../README.md")]

extern crate alloc;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use image as _;
#[cfg(test)]
use pollster as _;
#[cfg(test)]
use wgpu as _;

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
#[cfg(feature = "shell")]
pub mod shell;
pub mod style;
pub mod tasks;
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
#[cfg(feature = "tray")]
pub use crate::app::tray;
pub use crate::app::{
    AccessibilityTree, Anchor, AppConfig, AppResult, Error, Host, Painted, PanicPolicy,
    ResizeDirection, TreeRoot, WindowCommand, WindowOpts, audio, pty, request_exit, root,
};
#[cfg(any(feature = "desktop", feature = "render"))]
pub use crate::app::{
    DefaultFonts, TextTuning, set_default_fonts, set_text_tuning, text_tuning, use_system_fonts,
};
#[cfg(feature = "desktop")]
pub use crate::app::{ThemeWatcher, parse_theme, set_theme, watch_theme};

#[cfg(feature = "desktop")]
pub fn launch<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    clipboard::install_os();
    crate::app::launch(config, tree)
}

#[cfg(feature = "clipboard")]
pub mod clipboard;
pub mod input_glyphs;
pub mod testing;
pub use crate::widgets::{interaction, widget_dev};

#[cfg(feature = "render")]
pub mod render;

/// Application import surface. Product code uses `use sigil::prelude::*`.
///
/// The modules below it are public but move faster than the prelude.
pub mod prelude {
    pub use crate::animation::{
        AnimationDirection, AnimationFill, CubicBezier, Easing, Keyframe, Keyframes, SpringPreset,
    };
    pub use crate::animation_runtime::{AnimationConfig, Animator, MotionPrefs, Playback};
    pub use crate::events::prelude::*;
    pub use crate::kernel::prelude::*;
    pub use crate::layout::prelude::*;
    pub use crate::reactive::{
        EventChannel, Signal, batch, derived, derived_eq, effect, on_cleanup, signal,
    };
    pub use crate::style::{
        ColorAtom, ColorOrToken, Dimension as StyleDimension, Fill, Flex, FocusRing, Geometry,
        NodeStyleExt, NumericAtomKind, Px, Rgba8, Sizing, Spacing, StateStyle, StateVariants,
        Style, StyleAccess, StyleAtom, StyleState, StyleVariant, Styleable, Styled, Theme,
        ThemeValue, Tone, Transitions, Typography, Visual, classes,
    };
    pub use crate::tasks::{
        OwnerHandle, SignalWriter, Status, dispatch_to_owner, dispatcher, post, signal_writer,
        spawn,
    };

    #[cfg(feature = "audio")]
    pub use crate::Player;
    #[cfg(feature = "pty")]
    pub use crate::Session;
    #[cfg(feature = "clipboard")]
    pub use crate::clipboard::{OsClipboard, copy, install, paste};
    pub use crate::interaction::InteractionState;
    #[cfg(feature = "desktop")]
    pub use crate::launch;
    #[cfg(feature = "desktop")]
    pub use crate::open;
    pub use crate::reactive::Runtime;
    pub use crate::root;
    #[cfg(feature = "image-decode")]
    pub use crate::widgets::image_bytes;
    pub use crate::widgets::layout_ext::WidgetLayoutExt;
    pub use crate::widgets::patterns;
    pub use crate::widgets::{
        ButtonBuilder, ChartBuilder, ChartMark, CheckboxBuilder, ChipBuilder, ClipError, Clipboard,
        CodeEditBuilder, ColAlign, ColWidth, Combobox, CommandMenuBuilder, DRAG_THRESHOLD_PX,
        DROP_TARGET_CLASS, Date, DateFieldBuilder, Density, DiffBuilder, DiffLayout, DotBuilder,
        DragDropExt, DragState, DropdownBuilder, FctSpawn, FloaterStyle, FloatingTextBuilder,
        FuzzyMatch, GaugeBuilder, HintOverlayBuilder, HlSpan, IconBuilder, Identifiable,
        ImageBuilder, InteractiveBuilder, ListRowSlots, MenuBarBuilder, MenuBuilder, MenuItem,
        ModalBuilder, NumberFieldBuilder, NumericKind, Orientation, PanelBuilder, PathCmd,
        PathFill, PathNode, ProgressBuilder, RadioGroupBuilder, ScrollAnchor, ScrollBuilder,
        SelectionMode, SelectionState, Side, SliderBuilder, SplitterBuilder, SplitterOrientation,
        SubmenuBuilder, TableBuilder, TableColumn, TableState, TabsBuilder, TextEditBuilder,
        TextEditState, TextFieldBuilder, ToastEntry, ToastLevel, ToastQueue, ToggleBuilder,
        TooltipBuilder, TransportBuilder, TurnStripBuilder, VirtualListBuilder, app_shell, avatar,
        axis_drag, badge, button, card, chart, checkbox, chip, code_edit, color_field, combobox,
        command_menu, date_field, diff, divider_horizontal, divider_vertical, dot, drag_ghost,
        dropdown, empty_state, field, floating_text, fuzzy_score, gauge, hairline, heatmap,
        hint_bar, hint_overlay, html, icon, icon_button, image_rgba, interactive, kv_grid, kv_row,
        label, list_row, list_row_with, markdown, markdown_edit, menu, menu_bar, mermaid, modal,
        notice_band, number_field, panel, path, progress, radio_group, scroll, scroll_auto,
        section_label, sheet, sidebar, sidebar_width, slider, spacer, spinner, spinner_frame,
        splitter, status_bar, surface, table, tabs, terminal, text_edit, text_field, toast,
        toast_queue, toggle, tooltip, transport, turn_strip, validated_field, virtual_list,
    };
    pub use crate::{AppConfig, AppResult, Error, Host, Painted, TreeRoot};
    #[cfg(feature = "desktop")]
    pub use crate::{ThemeWatcher, parse_theme, set_theme, watch_theme};
}
