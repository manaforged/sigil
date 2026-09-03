#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]
#![doc = include_str!("../README.md")]

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

#[cfg(feature = "dialog")]
pub use sigil_app::dialog;
#[cfg(feature = "menu")]
pub use sigil_app::menu;
#[cfg(feature = "desktop")]
pub use sigil_app::open;
#[cfg(feature = "tray")]
pub use sigil_app::tray;
pub use sigil_app::{
    AccessibilityTree, Anchor, AppConfig, AppResult, Error, Host, Painted, PanicPolicy, Player,
    ResizeDirection, Session, TreeRoot, WindowCommand, WindowOpts, audio, pty, request_exit, root,
};
#[cfg(any(feature = "desktop", feature = "render"))]
pub use sigil_app::{
    DefaultFonts, TextTuning, set_default_fonts, set_text_tuning, text_tuning, use_system_fonts,
};

#[cfg(feature = "desktop")]
pub fn launch<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    clipboard::install_os();
    sigil_app::launch(config, tree)
}

pub use sigil_profile as profile;
#[cfg(feature = "tui")]
pub fn run_tui<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    clipboard::install_os();
    sigil_tui::run(config, tree)
}
#[cfg(feature = "tui")]
pub use sigil_tui::{Screen, every, present, step, work};

#[cfg(feature = "clipboard")]
pub mod clipboard;
pub mod input_glyphs;
pub mod testing;
pub use sigil_widgets as widgets;
pub use sigil_widgets::{interaction, widget_dev};

#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "render")]
pub mod text;

pub mod reactive {
    pub use sigil_reactive::{
        EventChannel, Owner, Runtime, Signal, batch, derived, derived_eq, effect, effect_named,
        on_cleanup, signal,
    };
}

pub mod prelude {
    pub use sigil_actions::{Action, DeriveAction, HandlerKey, Propagate};
    pub use sigil_events::prelude::*;
    pub use sigil_kernel::prelude::*;
    pub use sigil_layout::prelude::*;
    pub use sigil_reactive::{EventChannel, Signal, derived, effect, signal};
    pub use sigil_style::{
        ColorAtom, ColorOrToken, Fill, Flex, FocusRing, Geometry, NodeStyleExt, Rgba8, Sizing,
        Spacing, StateStyle, StateVariants, Style, StyleAccess, StyleAtom, StyleState,
        StyleVariant, Styleable, Styled, Theme, ThemeValue, Tone, Transitions, Typography, Visual,
        classes,
    };
    pub use sigil_tasks::{
        OwnerHandle, SignalWriter, Status, dispatch_to_owner, dispatcher, post, signal_writer,
        spawn,
    };

    pub use crate::clipboard::{copy, install, paste};
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
        CodeEditBuilder, ColAlign, ColWidth, Combobox, CommandMenuBuilder, DRAG_THRESHOLD_PX,
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
