use crate::events::Key;
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, container};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, signal};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{
    BORDER_WIDTH, Paint, PaintTarget, StateVariants, Style, StyleAtom, StyleSlot, Token, space,
};

use super::super::drag::{Axis, axis_drag};
use super::super::interactive::interactive;
use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::splitter::{HANDLE_PX, KEY_STEP, cached_width_atom, resize_keys, to_i16};

const SIDEBAR_DEFAULT: f32 = space(55);

static SIDEBAR_CHROME: &[StyleAtom] = &[
    Token::SurfaceSubtle.bg(),
    StyleAtom::BorderRight(BORDER_WIDTH),
    Token::Border.border(),
];

pub struct SidebarBuilder {
    child: Box<dyn IntoNode>,
    resizable: Option<(f32, f32)>,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

#[must_use]
pub fn sidebar(child: impl IntoNode + 'static) -> SidebarBuilder {
    SidebarBuilder {
        child: Box::new(child),
        resizable: None,
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl SidebarBuilder {
    #[must_use]
    pub const fn resizable(mut self, min: f32, max: f32) -> Self {
        self.resizable = Some((min, max));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let start = self
            .layout_overrides
            .width_hint()
            .unwrap_or(SIDEBAR_DEFAULT);
        let column = container()
            .layout(column_layout().grow(1.0).min_size(0.0, 0.0))
            .child(self.child);
        let mut root = container()
            .role(Role::Navigation)
            .class(SIDEBAR_CHROME)
            .child(column);
        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        match self.resizable {
            None => root.layout(self.layout_overrides.apply(root_layout()).width(start)),
            Some((min, max)) => {
                let width = signal(start.clamp(min, max.max(min)));
                root.layout(self.layout_overrides.apply(root_layout()))
                    .style_reactive(move || cached_width_atom(to_i16(width.get())))
                    .child(resize_handle(width, min, max.max(min)))
            }
        }
    }
}

fn root_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_height()
        .shrink(0.0)
        .align_items(AlignItems::Stretch)
}

fn column_layout() -> LayoutStyle {
    LayoutStyle::flex_column().padding(space(2)).gap(space(1))
}

fn resize_handle(width: Signal<f32>, min: f32, max: f32) -> NodeBuilder {
    let node = container()
        .role(Role::Splitter)
        .hittable(true)
        .focusable(true)
        .label(theme_label(StringKey::ResizeSidebar))
        .value(width, min, max)
        .value_step(KEY_STEP)
        .layout(
            LayoutStyle::block()
                .width(HANDLE_PX)
                .fill_height()
                .shrink(0.0),
        );
    let node = interactive(node)
        .hover(|s| s.bg(Token::SurfaceHover))
        .pressed(|s| s.bg(Token::SurfacePressed))
        .into_node_builder();
    let node = axis_drag(
        node,
        Axis::X,
        move || width.peek(),
        move |base, delta| width.set((base + delta).clamp(min, max)),
        || {},
    );
    resize_keys(node, (Key::ArrowLeft, Key::ArrowRight), move |step| {
        width.set((width.peek() + step).clamp(min, max));
    })
}

impl PaintTarget for SidebarBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for SidebarBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for SidebarBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}
