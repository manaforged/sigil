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
            fn set_state(
                mut self,
                slot: crate::style::StateSlot,
                atoms: smallvec::SmallVec<[crate::style::StyleAtom; 16]>,
            ) -> Self {
                self.$field.set_state(slot, atoms);
                self
            }
        }
    };
    (@widget [$($g:tt)*] $ty:ty $(, focus = $autofocus:ident / $trigger:ident)?) => {
        impl<$($g)*> crate::widgets::Sealed for $ty {
            fn edit(mut self, edit: crate::widgets::Edit) -> Self {
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
            fn edit(mut self, edit: crate::widgets::Edit) -> Self {
                self.$field = self.$field.map(move |inner| crate::widgets::Sealed::edit(inner, edit));
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
            fn set_state(
                mut self,
                slot: crate::style::StateSlot,
                atoms: smallvec::SmallVec<[crate::style::StyleAtom; 16]>,
            ) -> Self {
                self.$field = self.$field.map(move |inner| crate::widgets::StateTarget::set_state(inner, slot, atoms));
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
pub(crate) mod layout_ext;
mod listbox;
mod markdown;
pub(crate) mod math;
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
pub(crate) use popup::close_light as popup_close_light;
pub use popup::close_top_escape;
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

pub use button::{ButtonBuilder, button, icon_button};
pub(crate) use canvas::CanvasPaint;
pub use canvas::canvas;
pub use chart::{ChartBuilder, chart};
pub(crate) use checkbox::CheckboxBuilder;
pub use checkbox::checkbox;
pub use chip::chip;
pub(crate) use code_edit::CodeEditBuilder;
pub use code_edit::{Change, Diagnostic, Severity, code_edit};
pub use color_field::color_field;
pub(crate) use color_field::{ColorFieldBuilder, parse_hex as parse_color_hex};
#[cfg(all(feature = "scene", feature = "kittest"))]
#[doc(hidden)]
pub use svg_picture::{SvgMode, svg_scene};

pub use combobox::{Combobox, combobox};
pub use command_menu::command_menu;
kittest_pub! {
    #[doc(hidden)]
    pub use command_menu::CommandMenuBuilder;
}
pub use composition::{kv_row, status_bar};
pub(crate) use date_field::DateFieldBuilder;
pub use date_field::{Date, date_field};
pub use diff::{DiffBuilder, DiffView, Hunk, Line, Patch, Selection, diff};
pub use divider::{horizontal as divider_horizontal, vertical as divider_vertical};
pub(crate) use dot::DotBuilder;
pub use dot::dot;
pub use drag::axis_drag;
pub use drag_drop::{DragState, drag_ghost};
pub(crate) use dropdown::DropdownBuilder;
pub use dropdown::dropdown;
pub use field::field;
pub use fuzzy::fuzzy_score;
pub(crate) use gauge::GaugeBuilder;
pub use gauge::gauge;
pub use graphviz::graphviz;
pub use highlight::{Highlight, highlight};
pub use html::html;
#[cfg(feature = "scene")]
pub use icon::icon_svg;
pub use icon::{IconBuilder, icon};
pub use interactive::interactive;
pub use listbox::{CursorEdge, ListState};
pub(crate) use markdown::MarkdownEditBuilder;
pub use markdown::{markdown, markdown_edit, markdown_inline};
pub use math::math;
pub(crate) use menu::{MenuBarBuilder, MenuBuilder};
pub use menu::{MenuItem, context_menu, menu, menu_bar};
#[cfg(fuzzing)]
pub(crate) use mermaid::figure as mermaid_figure;
pub use mermaid::mermaid;
pub use modal::{ModalBuilder, Side, modal, sheet};
pub use number_field::number_field;
pub use orientation::Orientation;
pub use overlay::heatmap;
pub(crate) use panel::PanelBuilder;
pub use panel::panel;
pub use patterns::{
    Disclosure, ListColumn, ListRowSlots, RowTone, app_shell, avatar, card, disclosure,
    empty_state, hint_bar, hint_overlay, kv_grid, list_row, notice_band, section_label, sidebar,
    spacer,
};
pub(crate) use progress::ProgressBuilder;
pub use progress::progress;
pub(crate) use radio_group::RadioGroupBuilder;
pub use radio_group::radio_group;
pub use scroll::scroll;
pub(crate) use segmented::SegmentedBuilder;
pub use segmented::segmented;
pub(crate) use slider::SliderBuilder;
pub use slider::slider;
#[cfg(feature = "terminal")]
pub(crate) use sparkline::spark_glyph;
pub use sparkline::{SparkFill, SparkMark, SparklineBuilder, sparkline};
pub use spinner::{spinner, spinner_frame};
pub use splitter::splitter;
pub use stat::stat;
pub use surface::surface;
pub use table::{
    ColumnAlign, ColumnWidth, Density, Identifiable, SortDirection, SortKey, TableColumn,
    TableState, table,
};
kittest_pub! {
    #[doc(hidden)]
    pub use table::MIN_COL_WIDTH;
}
pub use a2ui::{A2uiBuilder, a2ui};
pub(crate) use tabs::TabsBuilder;
pub use tabs::tabs;
pub use terminal::terminal;
pub use text_edit::{SubmitKey, TextEditBuilder, TextEditState, text_edit};
pub use text_field::{TextFieldBuilder, text_field};
pub use toast::toast;
pub use toast_queue::{ToastQueue, toast_queue};
pub(crate) use toggle::ToggleBuilder;
pub use toggle::toggle;
pub(crate) use tooltip::TooltipBuilder;
pub use tooltip::tooltip;
pub use tree::{TreeNode, tree};
pub use vector::{PathNode, path};
pub use virtual_list::{
    RowRender, ScrollAnchor, SelectionMode, SelectionState, VirtualListBuilder, virtual_list,
};
pub use widget::Widget;
pub(crate) use widget::{Edit, LayoutEdit, Sealed, StateTarget};
