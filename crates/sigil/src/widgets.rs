#![allow(clippy::wildcard_imports)]

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

const MAX_SOURCE_LINES: usize = 256;
const MAX_SOURCE_CHARS: usize = 512;

pub(crate) fn bound_source(source: &str) -> String {
    let mut out = String::new();
    for (index, line) in source.lines().enumerate() {
        if index >= MAX_SOURCE_LINES {
            out.push_str("source truncated");
            break;
        }
        out.extend(line.chars().take(MAX_SOURCE_CHARS));
        out.push('\n');
    }
    out
}

pub(crate) fn frame_source(source: &str) -> Vec<String> {
    let mut lines: Vec<String> = source.trim_end().lines().map(ToString::to_string).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let bar = "─".repeat(width.saturating_add(2));
    let mut out = vec![format!("┌{bar}┐")];
    for line in lines {
        let pad = width.saturating_sub(line.chars().count());
        out.push(format!("│ {line}{} │", " ".repeat(pad)));
    }
    out.push(format!("└{bar}┘"));
    out
}

pub(crate) fn oversize_block() -> markdown::parse::Block {
    markdown::parse::Block::Paragraph(vec![markdown::parse::Inline::Text(oversize_message())])
}

macro_rules! builder_impls {
    ($gen:tt $ty:ty: $($part:ident $(($($arg:tt)*))?),+ $(,)?) => {
        $($crate::widgets::builder_impls!(@$part $gen $ty $(, $($arg)*)?);)+
    };
    (@paint [$($g:tt)*] $ty:ty, $field:ident) => {
        impl<$($g)*> crate::style::PaintTarget for $ty {
            fn style_slot(&mut self) -> crate::style::StyleSlot<'_> {
                crate::style::StyleSlot(&mut self.$field)
            }
        }
    };
    (@states [$($g:tt)*] $ty:ty, $field:ident) => {
        impl<$($g)*> crate::widgets::StateTarget for $ty {
            fn set_state(mut self, state: crate::widgets::StatePaint) -> Self {
                self.$field.set_state(state.slot, state.atoms);
                self
            }
        }
    };
    (@widget [$($g:tt)*] $ty:ty $(, focus = $autofocus:ident / $trigger:ident)?) => {
        impl<$($g)*> crate::widgets::Sealed for $ty {
            fn edit(mut self, crate::widgets::NodeEdit(edit): crate::widgets::NodeEdit) -> Self {
                match edit {
                    crate::widgets::Edit::Layout(edit) => self.layout_overrides.edit(edit),
                    $(
                        crate::widgets::Edit::Autofocus => self.$autofocus = true,
                        crate::widgets::Edit::FocusWhen(trigger) => self.$trigger = Some(trigger),
                    )?
                    #[allow(unreachable_patterns, reason = "builders that store every edit leave no rest")]
                    other => self.layout_overrides.node_hook(move |node| other.apply(node)),
                }
                self
            }

            fn map_target(
                self,
                f: impl FnOnce(crate::kernel::NodeBuilder) -> crate::kernel::NodeBuilder + 'static,
            ) -> Self {
                crate::widgets::Widget::map_node(self, f)
            }
        }

        impl<$($g)*> crate::widgets::Widget for $ty {
            fn map_node(
                mut self,
                f: impl FnOnce(crate::kernel::NodeBuilder) -> crate::kernel::NodeBuilder + 'static,
            ) -> Self {
                self.layout_overrides.node_hook(f);
                self
            }
        }
    };
    (@forward [$($g:tt)*] $ty:ty, $field:ident) => {
        impl<$($g)*> crate::widgets::Sealed for $ty {
            fn edit(mut self, crate::widgets::NodeEdit(edit): crate::widgets::NodeEdit) -> Self {
                self.$field = self.$field.map(move |inner| crate::widgets::Sealed::edit(inner, crate::widgets::NodeEdit(edit)));
                self
            }

            fn map_target(
                mut self,
                f: impl FnOnce(crate::kernel::NodeBuilder) -> crate::kernel::NodeBuilder + 'static,
            ) -> Self {
                self.$field = self.$field.map(move |inner| crate::widgets::Sealed::map_target(inner, f));
                self
            }
        }

        impl<$($g)*> crate::widgets::StateTarget for $ty {
            fn set_state(mut self, state: crate::widgets::StatePaint) -> Self {
                self.$field = self.$field.map(move |inner| crate::widgets::StateTarget::set_state(inner, state));
                self
            }
        }

        impl<$($g)*> crate::widgets::Widget for $ty {
            fn map_node(
                mut self,
                f: impl FnOnce(crate::kernel::NodeBuilder) -> crate::kernel::NodeBuilder + 'static,
            ) -> Self {
                self.$field = self.$field.map(move |inner| crate::widgets::Widget::map_node(inner, f));
                self
            }
        }
    };
    (@node [$($g:tt)*] $ty:ty $(, $vis:vis)?) => {
        impl<$($g)*> $ty {
            /// Builds the root node, then applies the queued layout edits and node changes.
            #[must_use]
            $($vis)? fn into_node_builder(self) -> crate::kernel::NodeBuilder {
                let overrides = self.layout_overrides.clone();
                overrides.finish(self.root())
            }
        }

        impl<$($g)*> crate::kernel::IntoNode for $ty {
            fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
                crate::kernel::IntoNode::build(self.into_node_builder(), cx)
            }
        }
    };
}

pub(crate) use builder_impls;

#[cfg(test)]
#[path = "widgets/lib_tests.rs"]
mod tests;

pub(crate) mod interaction;

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
mod graphviz;
mod highlight;
mod html;
mod icon;
mod interactive;
mod kernel_builders;
pub(crate) mod layout_ext;
mod listbox;
mod markdown;
pub(crate) mod math;
mod menu;
mod mermaid;
mod modal;
mod notice;
mod number_field;
pub(crate) mod overlay;
mod panel;
pub(crate) mod patterns;
mod popup;
pub(crate) use popup::close_light as popup_close_light;
pub use popup::close_top_escape;
pub(crate) use popup::{PopupStacks, swap_layers as popup_swap_layers};
#[cfg(feature = "a2ui")]
mod a2ui;
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
#[cfg(feature = "scene")]
mod svg_picture;
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
mod vega;
mod virtual_list;
mod widget;

pub use button::{ButtonBuilder, ButtonGlyph, button, icon_button};
pub use canvas::{CanvasBuilder, canvas};
pub use chart::{ChartBuilder, chart};
pub use checkbox::{CheckboxBuilder, checkbox};
pub use chip::{ChipBuilder, chip};
pub use code_edit::{Change, CodeEditBuilder, Diagnostic, Severity, code_edit};
pub use color_field::{ColorFieldBuilder, color_field};
#[cfg(all(feature = "scene", feature = "kittest"))]
#[doc(hidden)]
pub use svg_picture::{SvgMode, svg_scene};

pub use crate::layout::Orientation;
pub use combobox::{Combobox, combobox};
pub use command_menu::{CommandMenuBuilder, SubmenuBuilder, command_menu};
pub use composition::{kv_row, status_bar};
pub use date_field::{Date, DateFieldBuilder, date_field};
pub use diff::{
    DiffBuilder, DiffSource, DiffView, Hunk, Line, LineId, LineKind, Patch, Selection, diff,
};
pub use divider::divider;
pub use dot::{DotBuilder, dot};
pub use drag::axis_drag;
pub use drag_drop::{DragState, drag_ghost};
pub use dropdown::{DropdownBuilder, dropdown};
pub use field::{FieldBuilder, FieldControl, field};
pub use fuzzy::{FuzzyMatch, fuzzy_score};
pub use gauge::{GaugeBuilder, gauge};
pub use graphviz::graphviz;
pub use highlight::{Highlight, highlight};
pub use html::html;
#[cfg(feature = "scene")]
pub use icon::icon_svg;
pub use icon::{IconBuilder, IconSource, IntoIconSource, icon};
pub(crate) use interactive::{Wiring, wire};
pub use listbox::{CursorEdge, ListState};
pub use markdown::{MarkdownEditBuilder, markdown, markdown_edit, markdown_inline};
pub use math::math;
pub use menu::{MenuBarBuilder, MenuBuilder, MenuItem, context_menu, menu, menu_bar};
#[cfg(fuzzing)]
pub(crate) use mermaid::figure as mermaid_figure;
pub use mermaid::mermaid;
pub use modal::{ModalBuilder, Side, modal, sheet};
pub use number_field::{NumberFieldBuilder, NumericKind, number_field};
pub use overlay::heatmap;
pub use panel::{PanelBuilder, panel};
pub use patterns::{
    CardBuilder, Disclosure, HintOverlayBuilder, ListColumn, ListRowSlots, RowTone, SidebarBuilder,
    app_shell, avatar, card, disclosure, empty_state, hint_bar, hint_overlay, kv_grid, list_row,
    notice_band, section_label, sidebar, spacer,
};
pub use progress::{ProgressBuilder, progress};
pub use radio_group::{RadioGroupBuilder, radio_group};
pub use scroll::{IntoScrollExtent, ScrollBuilder, scroll};
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
    ColumnAlign, ColumnWidth, Density, Identifiable, SortDirection, SortKey, TableBuilder,
    TableColumn, TableState, table,
};
kittest_pub! {
    #[doc(hidden)]
    pub use table::MIN_COL_WIDTH;
}
#[cfg(feature = "a2ui")]
pub use a2ui::{A2UI_CATALOG, A2uiBuilder, a2ui};
pub use tabs::{TabsBuilder, tabs};
pub use terminal::terminal;
pub use text_edit::{SubmitKey, TextEditBuilder, TextEditState, text_edit};
pub use text_field::{TextFieldBuilder, text_field};
pub use toast::toast;
pub use toast_queue::{ToastQueue, toast_queue};
pub use toggle::{ToggleBuilder, toggle};
pub use tooltip::{TooltipBuilder, tooltip};
pub use tree::{TreeBuilder, TreeNode, tree};
pub use vector::{PathNode, path};
pub use virtual_list::{
    IntoList, RowRender, ScrollAnchor, SelectionMode, SelectionState, VirtualListBuilder,
    virtual_list,
};
pub use widget::Widget;
kittest_pub! {
    #[doc(hidden)]
    pub use widget::Layout;
}
pub(crate) use widget::{Edit, LayoutEdit, NodeEdit, Sealed, StatePaint, StateTarget};

crate::kernel::into_node_builder!(
    ButtonBuilder,
    ChartBuilder,
    CheckboxBuilder,
    CodeEditBuilder,
    ColorFieldBuilder,
    CommandMenuBuilder,
    DateFieldBuilder,
    DiffBuilder,
    DotBuilder,
    DropdownBuilder,
    GaugeBuilder,
    IconBuilder,
    MarkdownEditBuilder,
    MenuBarBuilder,
    MenuBuilder,
    ModalBuilder,
    PanelBuilder,
    PathNode,
    ProgressBuilder,
    RadioGroupBuilder,
    SegmentedBuilder,
    SliderBuilder,
    SparklineBuilder,
    TabsBuilder,
    TextEditBuilder,
    TextFieldBuilder,
    ToggleBuilder,
    TooltipBuilder,
);
