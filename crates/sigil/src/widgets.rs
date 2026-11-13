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

pub(crate) fn scrolled_diagram(
    diagram: crate::kernel::NodeBuilder,
    width: f32,
) -> crate::kernel::NodeBuilder {
    let centred = crate::kernel::container()
        .layout(
            crate::layout::LayoutStyle::flex_row()
                .fill_width()
                .min_size(width, 0.0)
                .justify_content(crate::layout::JustifyContent::Center),
        )
        .child(diagram);
    scroll::Horizontal::new(crate::reactive::signal(width), None).wrap(centred)
}

pub(crate) fn lines_extent(lines: &[String]) -> f32 {
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    f32::from(u16::try_from(width).unwrap_or(u16::MAX)) * crate::style::space(2)
}

pub(crate) fn framed_figure(
    source: &str,
    label: impl Into<String>,
    lines: &[String],
) -> crate::kernel::NodeBuilder {
    let extent = lines_extent(lines);
    let mut col = crate::kernel::container()
        .layout(crate::layout::LayoutStyle::flex_column().fill_width())
        .class("font-mono text-sm");
    for line in lines {
        col = col.child(crate::kernel::text(line.clone()).nowrap());
    }
    scroll::Horizontal::new(crate::reactive::signal(extent), None)
        .wrap(col)
        .role(crate::kernel::Role::Figure)
        .label(label.into())
        .description(bound_source(source).trim_end().to_string())
}

macro_rules! widget_layout_methods {
    (@edit $($path:tt)+) => {
        fn layout(mut self, style: $crate::layout::LayoutStyle) -> Self {
            self.$($path)+($crate::widgets::LayoutEdit::Merge(style));
            self
        }

        fn width(mut self, width: impl Into<$crate::layout::Length>) -> Self {
            self.$($path)+($crate::widgets::LayoutEdit::Width(width.into()));
            self
        }

        fn height(mut self, height: impl Into<$crate::layout::Length>) -> Self {
            self.$($path)+($crate::widgets::LayoutEdit::Height(height.into()));
            self
        }

        fn grow(mut self, factor: f32) -> Self {
            self.$($path)+($crate::widgets::LayoutEdit::Grow(factor));
            self
        }

        $crate::layout::builder_ext::layout_methods!(edit $($path)+);
    };
    (@kernel) => {
        fn layout(mut self, style: $crate::layout::LayoutStyle) -> Self {
            self.node = self.node.mutate(move |n| {
                $crate::layout::builder_ext::install_layout(n, &style)
            });
            self
        }

        fn width(mut self, width: impl Into<$crate::layout::Length>) -> Self {
            let edit = $crate::widgets::LayoutEdit::Width(width.into());
            self.node = self.node.mutate(move |n| edit.install(n));
            self
        }

        fn height(mut self, height: impl Into<$crate::layout::Length>) -> Self {
            let edit = $crate::widgets::LayoutEdit::Height(height.into());
            self.node = self.node.mutate(move |n| edit.install(n));
            self
        }

        fn grow(mut self, factor: f32) -> Self {
            let edit = $crate::widgets::LayoutEdit::Grow(factor);
            self.node = self.node.mutate(move |n| edit.install(n));
            self
        }

        $crate::layout::builder_ext::layout_methods!(kernel);
    };
    (@forward $field:ident) => {
        fn layout(mut self, style: $crate::layout::LayoutStyle) -> Self {
            self.$field = self.$field.map(move |inner| $crate::widgets::Widget::layout(inner, style));
            self
        }

        fn width(mut self, width: impl Into<$crate::layout::Length>) -> Self {
            let width = width.into();
            self.$field = self.$field.map(move |inner| $crate::widgets::Widget::width(inner, width));
            self
        }

        fn height(mut self, height: impl Into<$crate::layout::Length>) -> Self {
            let height = height.into();
            self.$field = self.$field.map(move |inner| $crate::widgets::Widget::height(inner, height));
            self
        }

        fn grow(mut self, factor: f32) -> Self {
            self.$field = self.$field.map(move |inner| $crate::widgets::Widget::grow(inner, factor));
            self
        }

        $crate::layout::builder_ext::layout_methods!(forward $field);
    };
}

macro_rules! forward_control_methods {
    (direct $field:ident) => {
        $crate::widgets::forward_control_methods!(@inner direct $field);
    };
    (map $field:ident) => {
        $crate::widgets::forward_control_methods!(@inner map $field);
    };
    (@inner $mode:ident $field:ident) => {
        fn class(mut self, classes: &'static str) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, class(classes));
            self
        }

        fn theme(mut self, theme: $crate::style::Theme) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, theme(theme));
            self
        }

        fn tone_scope(mut self, tone: $crate::style::Tone) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, tone_scope(tone));
            self
        }

        fn focus_ring(mut self, ring: impl Into<$crate::style::FocusRing>) -> Self {
            let ring = ring.into();
            $crate::widgets::forward_control_methods!(@call $mode self.$field, focus_ring(ring));
            self
        }

        fn presence(
            mut self,
            shown: $crate::reactive::Signal<bool>,
            ms: u16,
            easing: $crate::animation::Easing,
        ) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, presence(shown, ms, easing));
            self
        }

        fn drive(
            mut self,
            property: fn(f32) -> $crate::animation::Property,
            value: impl $crate::reactive::Reactive<f32>,
        ) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, drive(property, value));
            self
        }

        fn visible(mut self, shown: impl $crate::reactive::Reactive<bool>) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, visible(shown));
            self
        }

        fn paint_when<F>(mut self, active: impl $crate::reactive::Reactive<bool>, paint: F) -> Self
        where
            F: FnOnce($crate::style::StateStyleBuilder) -> $crate::style::StateStyleBuilder,
        {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, paint_when(active, paint));
            self
        }

        fn autofocus(mut self) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, autofocus());
            self
        }

        fn focus_when(mut self, trigger: $crate::reactive::Signal<u64>) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, focus_when(trigger));
            self
        }

        fn track_hovered(mut self, out: $crate::reactive::Signal<bool>) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, track_hovered(out));
            self
        }

        fn track_focused(mut self, out: $crate::reactive::Signal<bool>) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, track_focused(out));
            self
        }

        fn track_pressed(mut self, out: $crate::reactive::Signal<bool>) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, track_pressed(out));
            self
        }

        fn pointer_drag(
            mut self,
            handler: impl FnMut($crate::events::DragPhase, &mut $crate::events::PointerEvent) + 'static,
        ) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, pointer_drag(handler));
            self
        }

        fn a11y_edit(
            mut self,
            handler: impl Fn(&$crate::events::A11yEdit) -> bool + 'static,
        ) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, a11y_edit(handler));
            self
        }

        fn capture_text_style(mut self, out: $crate::reactive::Signal<$crate::style::TextStyle>) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, capture_text_style(out));
            self
        }

        fn ime_caret(
            mut self,
            caret: impl $crate::reactive::Reactive<Option<$crate::text::CaretGeometry>>,
        ) -> Self {
            $crate::widgets::forward_control_methods!(@call $mode self.$field, ime_caret(caret));
            self
        }
    };
    (@call direct $target:expr, $method:ident($($arg:expr),*)) => {
        $target = $crate::widgets::Widget::$method($target $(, $arg)*);
    };
    (@call map $target:expr, $method:ident($($arg:expr),*)) => {
        $target = $target.map(move |inner| $crate::widgets::Widget::$method(inner $(, $arg)*));
    };
}

macro_rules! builder_impls {
    ($gen:tt $ty:ty: $($part:ident $(($($arg:tt)*))?),+ $(,)?) => {
        $($crate::widgets::builder_impls!(@$part $gen $ty $(, $($arg)*)?);)+
    };
    (@paint [$($g:tt)*] $ty:ty, $field:ident) => {
        impl<$($g)*> crate::style::Paint for $ty {
            fn paint_mut(&mut self) -> &mut crate::style::StateStyleBuilder {
                self.$field.paint_mut()
            }
        }
    };
    (@states [$($g:tt)*] $ty:ty, $field:ident) => {
        impl<$($g)*> crate::style::States for $ty {
            $crate::style::state_methods!(style $field);
        }
    };
    (@widget [$($g:tt)*] $ty:ty $(, focus = $autofocus:ident / $trigger:ident)?) => {
        impl<$($g)*> crate::widgets::Widget for $ty {
            fn map_node(
                mut self,
                f: impl FnOnce(crate::kernel::NodeBuilder) -> crate::kernel::NodeBuilder + 'static,
            ) -> Self {
                self.layout_overrides.node_hook(f);
                self
            }

            $crate::widgets::widget_layout_methods!(@edit layout_overrides.edit);

            $(
                fn autofocus(mut self) -> Self {
                    self.$autofocus = true;
                    self
                }

                fn focus_when(mut self, trigger: crate::reactive::Signal<u64>) -> Self {
                    self.$trigger = Some(trigger);
                    self
                }
            )?
        }
    };
    (@kernel [$($g:tt)*] $ty:ty) => {
        impl<$($g)*> crate::widgets::Widget for $ty {
            fn map_node(
                mut self,
                f: impl FnOnce(crate::kernel::NodeBuilder) -> crate::kernel::NodeBuilder + 'static,
            ) -> Self {
                self.hooks
                    .push(Box::new(move |node| node.map_keeping_kind(f)));
                self
            }

            $crate::widgets::widget_layout_methods!(@kernel);

            fn autofocus(mut self) -> Self {
                self.node = crate::events::builder_ext::autofocus(self.node);
                self
            }

            fn focus_when(mut self, trigger: crate::reactive::Signal<u64>) -> Self {
                self.node = crate::events::builder_ext::focus_when(self.node, trigger);
                self
            }
        }
    };
    (@forward [$($g:tt)*] $ty:ty, $field:ident) => {
        impl<$($g)*> crate::style::Paint for $ty {
            fn paint_mut(&mut self) -> &mut crate::style::StateStyleBuilder {
                self.$field.paint_mut()
            }
        }

        impl<$($g)*> crate::style::States for $ty {
            $crate::style::state_methods!(map $field);
        }

        impl<$($g)*> crate::widgets::Widget for $ty {
            fn map_node(
                mut self,
                f: impl FnOnce(crate::kernel::NodeBuilder) -> crate::kernel::NodeBuilder + 'static,
            ) -> Self {
                self.$field = self.$field.map(move |inner| crate::widgets::Widget::map_node(inner, f));
                self
            }

            $crate::widgets::widget_layout_methods!(@forward $field);

            fn autofocus(mut self) -> Self {
                self.$field = self.$field.map(crate::widgets::Widget::autofocus);
                self
            }

            fn focus_when(mut self, trigger: crate::reactive::Signal<u64>) -> Self {
                self.$field = self.$field.map(move |inner| crate::widgets::Widget::focus_when(inner, trigger));
                self
            }
        }
    };
    (@node [$($g:tt)*] $ty:ty $(, $vis:vis)?) => {
        impl<$($g)*> $ty {
            /// Builds the root node, then applies the queued layout edits and node changes.
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

pub(crate) use {builder_impls, forward_control_methods, widget_layout_methods};

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
mod frame;
mod fuzzy;
mod gauge;
mod graphviz;
mod highlight;
mod html;
mod icon;
#[cfg(feature = "image-decode")]
mod image_load;
mod interactive;
mod kernel_builders;
pub(crate) mod layout_ext;
mod link;
mod listbox;
mod log_view;
mod markdown;
mod math;
mod menu;
mod mermaid;
mod modal;
mod notice;
mod number_field;
pub(crate) mod overlay;
pub(crate) mod patterns;
mod popup;
pub(crate) use popup::close_light as popup_close_light;
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
mod tag_input;
mod terminal;
mod text_edit;
mod text_field;
mod text_search;
mod time_field;
mod toast;
mod toast_queue;
mod toggle;
mod tooltip;
mod tree;
mod vector;
mod vega;
mod virtual_list;
mod widget;

pub use button::{ButtonBuilder, button};
pub use canvas::{CanvasBuilder, canvas};
pub use chart::{ChartBuilder, chart};
pub use checkbox::{CheckboxBuilder, checkbox};
pub use chip::{ChipBuilder, chip};
pub use code_edit::{Change, CodeEditBuilder, Diagnostic, Severity, code_edit};
pub use color_field::{ColorFieldBuilder, color_field};
#[cfg(feature = "image-decode")]
pub use image_load::{ImageLoadBuilder, load_image};
#[cfg(scene_harness)]
#[doc(hidden)]
pub use svg_picture::{SvgMode, svg_scene};

pub(crate) use crate::layout::Orientation;
pub use combobox::{Combobox, combobox};
pub use command_menu::{CommandMenuBuilder, SubmenuBuilder, command_menu};
pub use composition::{kv_row, status_bar};
pub use date_field::{Date, DateFieldBuilder, date_field};
pub use diff::{
    DiffBuilder, DiffSelection, DiffSource, DiffView, Hunk, Line, LineId, LineKind, Patch, diff,
};
pub use divider::divider;
pub use dot::{DotBuilder, dot};
pub use drag::axis_drag;
pub use drag_drop::{DragState, drag_ghost};
pub use dropdown::{DropdownBuilder, dropdown};
pub use field::{FieldBuilder, FieldControl, field};
pub use frame::{Framed, WidgetFrame};
pub use fuzzy::{FuzzyMatch, fuzzy_score};
pub use gauge::{GaugeBuilder, gauge};
pub use graphviz::graphviz;
pub use highlight::{Highlight, highlight};
pub use html::html;
pub use icon::{Glyph, IconBuilder, icon};
pub(crate) use interactive::{Wiring, wire};
pub use link::{LinkBuilder, link};
pub use listbox::{CursorEdge, ListState};
pub use log_view::{Log, log_view};
pub use markdown::{MarkdownEditBuilder, markdown, markdown_edit, markdown_inline};
pub use math::math;
pub use menu::{MenuBarBuilder, MenuBuilder, MenuItem, context_menu, menu, menu_bar};
#[cfg(fuzzing)]
pub(crate) use mermaid::figure as mermaid_figure;
pub use mermaid::mermaid;
pub use modal::{ModalBuilder, Side, modal};
pub use number_field::{NumberFieldBuilder, NumericKind, number_field};
pub use overlay::heatmap;
pub use patterns::{
    CardBuilder, Disclosure, HintOverlayBuilder, ListRowBuilder, RowTone, SidebarBuilder,
    app_shell, avatar, badge, breadcrumbs, card, disclosure, empty_state, hint_bar, hint_overlay,
    kv_grid, list_row, notice_band, pagination, section_label, sidebar, skeleton, spacer, stepper,
};
pub use progress::{ProgressBuilder, progress};
pub use radio_group::{RadioGroupBuilder, radio_group};
pub use scroll::{ScrollBuilder, scroll};
pub use segmented::{SegmentedBuilder, segmented};
pub use slider::{SliderBuilder, range_slider, slider};
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
pub use tag_input::{TagInputBuilder, tag_input};
pub use terminal::terminal;
pub use text_edit::{SubmitKey, TextState};
pub use text_field::{TextFieldBuilder, text_field};
pub use time_field::{DateTime, Time, TimeFieldBuilder, TimeValue, time_field};
pub use toast::toast;
pub use toast_queue::{ToastQueue, toast_queue};
pub use toggle::{ToggleBuilder, toggle};
pub use tooltip::{TooltipBuilder, tooltip};
pub use tree::{TreeBuilder, TreeNode, tree};
pub use vector::{PathBuilder, path};
pub use vega::vega;
pub use virtual_list::{
    IntoList, RowRender, ScrollAnchor, Selection, SelectionMode, VirtualListBuilder, virtual_list,
};
pub(crate) use widget::LayoutEdit;
pub use widget::Widget;

crate::kernel::into_node_builder!(
    ButtonBuilder,
    CanvasBuilder,
    ChartBuilder,
    CheckboxBuilder,
    ChipBuilder,
    CodeEditBuilder,
    ColorFieldBuilder,
    CommandMenuBuilder,
    DateFieldBuilder,
    DiffBuilder,
    DotBuilder,
    DropdownBuilder,
    GaugeBuilder,
    HintOverlayBuilder,
    IconBuilder,
    LinkBuilder,
    MarkdownEditBuilder,
    MenuBarBuilder,
    MenuBuilder,
    ModalBuilder,
    PathBuilder,
    ProgressBuilder,
    RadioGroupBuilder,
    SegmentedBuilder,
    SidebarBuilder,
    SparklineBuilder,
    TabsBuilder,
    TextFieldBuilder,
    ToggleBuilder,
    TooltipBuilder,
);

#[cfg(feature = "image-decode")]
impl From<ImageLoadBuilder> for crate::kernel::NodeBuilder {
    fn from(builder: ImageLoadBuilder) -> Self {
        Self::wrapping(builder)
    }
}

macro_rules! generic_into_node_builder {
    ($([$($g:tt)*] $ty:ty),+ $(,)?) => {
        $(
            impl<$($g)*> From<$ty> for crate::kernel::NodeBuilder {
                fn from(builder: $ty) -> Self {
                    Self::wrapping(builder)
                }
            }
        )+
    };
}

generic_into_node_builder!(
    [C: crate::kernel::IntoNode + 'static] FieldBuilder<C>,
    [T: NumericKind] NumberFieldBuilder<T>,
    [T: 'static] SliderBuilder<T>,
    [Row: 'static] TableBuilder<Row>,
    [T: Clone + PartialEq + std::fmt::Display + 'static] TagInputBuilder<T>,
    [T: TimeValue] TimeFieldBuilder<T>,
    [K: Clone + Eq + std::hash::Hash + 'static] TreeBuilder<K>,
    [T: 'static, R: RowRender<T>] VirtualListBuilder<T, R>,
);
