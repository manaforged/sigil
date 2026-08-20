//! First-party Sigil widgets. Host-agnostic: no window, no GPU.
//!
//! Built from kernel, layout, style, events, and reactive state.

#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]
#![deny(missing_docs)]

// Doctests run against the facade's consumer path (`sigil::prelude::*`);
// reference the dev-dependency so the lib-test target does not flag it.
#[cfg(test)]
use sigil as _;

/// Shared hover/press/focus state for focusable widgets.
pub mod interaction;
/// Widget-author prelude for built-in and extension widgets.
pub mod widget_dev;

mod button;
mod chart;
mod checkbox;
mod chip;
mod code_edit;
mod color_field;
mod command_menu;
mod composition;
mod date_field;
mod divider;
mod dot;
mod drag;
mod dropdown;
mod floating_text;
mod gauge;
mod icon;
mod image;
pub mod interactive;
pub mod layout_ext;
mod markdown;
mod menu;
mod mermaid;
mod modal;
pub mod number_field;
pub mod overlay;
mod panel;
pub mod patterns;
mod progress;
mod radio_group;
mod scroll;
mod shared;
mod slider;
mod splitter;
mod table;
mod tabs;
mod text_edit;
mod text_field;
mod toast;
mod toggle;
mod tooltip;
pub mod transition_seam;
mod turn_strip;
mod vector;
mod virtual_list;

pub use button::{ButtonBuilder, button};
pub use chart::{ChartBuilder, ChartMark, chart};
pub use checkbox::{CheckboxBuilder, checkbox};
pub use chip::{ChipBuilder, badge, chip};
pub use code_edit::{ChangeKind, CodeEditBuilder, DiagSeverity, DiagSpan, HlSpan, code_edit};
pub use color_field::{color_field, parse_hex, to_hex};
pub use command_menu::{CommandMenuBuilder, SubmenuBuilder, command_menu};
pub use composition::{kv_row, status_bar};
pub use date_field::{Date, DateFieldBuilder, date_field};
pub use divider::{horizontal as divider_horizontal, vertical as divider_vertical};
pub use dot::{DotBuilder, dot};
pub use drag::{Axis, axis_drag};
pub use dropdown::{DropdownBuilder, dropdown};
pub use floating_text::{
    FCT_POOL_CAPACITY, FctNode, FctSpawn, FctState, FloaterStyle, FloatingTextBuilder, Particle,
    floating_text,
};
pub use gauge::{GaugeBuilder, gauge};
pub use icon::{IconBuilder, icon};
#[cfg(feature = "image-decode")]
pub use image::image_bytes;
pub use image::{ImageBuilder, image_rgba};
pub use interactive::{InteractiveBuilder, interactive};
pub use markdown::{markdown, markdown_edit};
pub use menu::{MenuBuilder, MenuItem, menu};
pub use mermaid::{draw as mermaid_draw, mermaid};
pub use modal::{ModalBuilder, Side, modal, sheet};
pub use number_field::{NumberFieldBuilder, NumericKind, number_field};
pub use overlay::heatmap;
pub use panel::{PanelBuilder, panel};
pub use patterns::{HintOverlayBuilder, field, validated_field};
pub use progress::{ProgressBuilder, progress};
pub use radio_group::{RadioGroupBuilder, radio_group};
pub use scroll::{ScrollBuilder, scroll};
pub use shared::label;
pub use slider::{Orientation, SliderBuilder, slider};
pub use splitter::{PaneBuilder, SplitterBuilder, SplitterOrientation, splitter};
pub use table::{
    ColWidth, ColumnLayout, Density, Identifiable, MIN_COL_WIDTH, SortDir, SortKey, TableBuilder,
    TableColumn, TableState, table,
};
pub use tabs::{TabsBuilder, tabs};
pub use text_edit::{TextEditBuilder, text_edit};
pub use text_field::{Clipboard, TextFieldBuilder, text_field};
pub use toast::toast;
pub use toggle::{ToggleBuilder, toggle};
pub use tooltip::{TooltipBuilder, tooltip};
pub use turn_strip::{TurnStripBuilder, turn_strip};
pub use vector::{PathCmd, PathFill, PathNode, path};
pub use virtual_list::{
    IntoList, ScrollAnchor, SelectionMode, SelectionState, VirtualListBuilder, virtual_list,
    virtual_list_bind,
};
