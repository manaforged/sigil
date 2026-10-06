#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

pub(crate) const MAX_SOURCE_BYTES: usize = 1 << 20;

pub(crate) const fn source_too_large(source: &str) -> bool {
    source.len() > MAX_SOURCE_BYTES
}

pub(crate) fn oversize_message() -> String {
    format!(
        "Source is larger than the {} MiB limit.",
        MAX_SOURCE_BYTES >> 20
    )
}

pub(crate) fn oversize_block() -> markdown::parse::Block {
    markdown::parse::Block::Paragraph(oversize_message())
}

#[cfg(test)]
#[path = "widgets/lib_tests.rs"]
mod tests;

pub mod interaction;

mod button;
mod canvas;
mod chart;
mod checkbox;
mod chip;
mod code_edit;
mod color_field;
mod combobox;
mod command_menu;
mod composition;
mod date_field;
mod diff;
mod divider;
mod dot;
mod drag;
mod drag_drop;
mod dropdown;
mod field;
mod fuzzy;
mod gauge;
mod grapheme;
mod highlight;
mod html;
mod icon;
mod image;
mod interactive;
pub mod layout_ext;
mod listbox;
mod markdown;
mod menu;
mod mermaid;
mod modal;
mod notice;
mod number_field;
mod orientation;
pub(crate) mod overlay;
mod panel;
pub(crate) mod patterns;
mod popup;
mod progress;
mod radio_group;
mod scroll;
mod segmented;
mod shared;
mod slider;
mod sparkline;
mod spinner;
mod splitter;
mod stat;
mod surface;
mod table;
mod tabs;
mod terminal;
mod text_edit;
mod text_field;
mod text_search;
mod toast;
mod toast_queue;
mod toggle;
mod tooltip;
mod tree;
mod vector;
mod virtual_list;

pub use button::{ButtonBuilder, ButtonGlyph, button, icon_button};
pub(crate) use canvas::CanvasPaint;
pub use canvas::{CanvasBuilder, canvas};
pub use chart::{ChartBuilder, ChartMark, chart};
pub use checkbox::{CheckboxBuilder, checkbox};
pub use chip::{ChipBuilder, chip};
pub use code_edit::{ChangeKind, CodeEditBuilder, DiagSeverity, DiagSpan, HlSpan, code_edit};
pub use color_field::{ColorFieldBuilder, color_field, parse_hex, to_hex};
pub use combobox::{Combobox, combobox};
pub use command_menu::{CommandMenuBuilder, SubmenuBuilder, command_menu};
pub use composition::{kv_row, status_bar};
pub use date_field::{Date, DateFieldBuilder, date_field};
pub use diff::{
    DiffBuilder, DiffSource, DiffView, Hunk, Line, LineId, LineKind, Patch, Selection, diff,
};
pub use divider::{
    VerticalDivider, horizontal as divider_horizontal, vertical as divider_vertical,
};
pub use dot::{DotBuilder, dot};
pub use drag::{Axis, axis_drag};
pub use drag_drop::{DRAG_THRESHOLD_PX, DROP_TARGET_CLASS, DragDropExt, DragState, drag_ghost};
pub use dropdown::{DropdownBuilder, dropdown};
pub use field::{FieldBuilder, FieldControl, field};
pub use fuzzy::{FuzzyMatch, fuzzy_score};
pub use gauge::{GaugeBuilder, gauge};
pub use highlight::{Highlight, highlight, hl_spans, language_for_path, text_runs};
pub use html::html;
#[cfg(feature = "scene")]
pub use icon::icon_svg;
pub use icon::{IconBuilder, IconSource, IntoIconSource, icon};
#[cfg(feature = "image-decode")]
pub use image::image_bytes;
pub use image::image_rgba;
pub use interactive::{InteractiveBuilder, interactive};
pub use markdown::{MarkdownEditBuilder, markdown, markdown_edit, markdown_inline};
pub use menu::{MenuBarBuilder, MenuBuilder, MenuItem, context_menu, menu, menu_bar};
#[cfg(fuzzing)]
pub(crate) use mermaid::figure as mermaid_figure;
pub use mermaid::mermaid;
pub use modal::{ModalBuilder, Side, modal, sheet};
pub use number_field::{NumberFieldBuilder, NumericKind, number_field};
pub use orientation::Orientation;
pub use overlay::heatmap;
pub use panel::{PanelBuilder, panel};
pub use patterns::{
    CardBuilder, Disclosure, HintEntry, HintOverlayBuilder, ListColumn, ListRowSlots, RowTone,
    SidebarBuilder, app_shell, avatar, card, empty_state, hint_bar, hint_overlay, kv_grid,
    list_row, list_row_with, notice_band, section_label, sidebar, spacer,
};
pub use progress::{ProgressBuilder, progress};
pub use radio_group::{RadioGroupBuilder, radio_group};
pub use scroll::{ScrollBuilder, scroll};
pub use segmented::{SegmentedBuilder, segmented};
pub use slider::{SliderBuilder, slider};
#[cfg(feature = "terminal")]
pub(crate) use sparkline::spark_glyph;
pub use sparkline::{SparkFill, SparkMark, SparklineBuilder, sparkline};
pub use spinner::{spinner, spinner_frame};
pub use splitter::{PaneBuilder, SplitterBuilder, splitter};
pub use stat::{StatBuilder, stat};
pub use surface::surface;
pub use table::{
    ColAlign, ColWidth, ColumnLayout, Density, Identifiable, MIN_COL_WIDTH, SortDir, SortKey,
    TableBuilder, TableColumn, TableState, table,
};
pub use tabs::{TabsBuilder, tabs};
pub use terminal::terminal;
pub use text_edit::{FocusCaret, SubmitKey, TextEditBuilder, TextEditState, text_edit};
pub use text_field::{TextFieldBuilder, text_field};
pub use toast::toast;
pub use toast_queue::{MAX_VISIBLE, TOAST_TTL, ToastEntry, ToastLevel, ToastQueue, toast_queue};
pub use toggle::{ToggleBuilder, toggle};
pub use tooltip::{TooltipBuilder, tooltip};
pub use tree::{TreeBuilder, TreeKey, TreeNode, TreeRow, tree};
pub use vector::{PathCmd, PathColor, PathFill, PathNode, path};
pub use virtual_list::{
    IntoList, RowRender, ScrollAnchor, SelectionMode, SelectionState, VirtualListBuilder,
    virtual_list, virtual_list_bind,
};
