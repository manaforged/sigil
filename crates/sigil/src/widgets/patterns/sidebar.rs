use crate::events::NamedKey;
use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::Space;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::{Signal, signal};
use crate::style::States;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{BORDER_WIDTH, Paint, Style, StyleAtom, Token, space};
use crate::widgets::Layout;

use super::super::drag::axis_drag;
use super::super::layout_ext::LayoutOverrides;
use super::super::shared::caller_style;
use super::super::splitter::{HANDLE_PX, KEY_STEP, cached_width_atom, resize_keys, to_i16};
use crate::layout::Orientation;

const SIDEBAR_DEFAULT: f32 = space(55);

static SIDEBAR_CHROME: &[StyleAtom] = &[
    Token::SurfaceSubtle.background(),
    StyleAtom::BorderRight(BORDER_WIDTH),
    Token::Border.border(),
];

/// Builder of a sidebar.
#[must_use]
pub struct SidebarBuilder {
    child: Box<dyn IntoNode>,
    resizable: Option<(f32, f32)>,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

impl core::fmt::Debug for SidebarBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SidebarBuilder")
            .field("resizable", &self.resizable)
            .field("layout_overrides", &self.layout_overrides)
            .field("style_accumulator", &self.style_accumulator)
            .finish_non_exhaustive()
    }
}

/// A side column.
pub fn sidebar(child: impl IntoNode + 'static) -> SidebarBuilder {
    SidebarBuilder {
        child: Box::new(child),
        resizable: None,
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl SidebarBuilder {
    /// Sets the resizable.
    pub const fn resizable(mut self, min: f32, max: f32) -> Self {
        self.resizable = Some((min, max));
        self
    }

    fn root(self) -> NodeBuilder {
        let start = self
            .layout_overrides
            .width_hint()
            .unwrap_or(SIDEBAR_DEFAULT);
        let column = container()
            .layout(
                LayoutStyle::flex_column()
                    .padding(Space::Sm)
                    .gap(Space::Xs)
                    .grow(1.0)
                    .min_size(0.0, 0.0),
            )
            .child(self.child);
        let root = container()
            .role(Role::Navigation)
            .class(SIDEBAR_CHROME)
            .child(column);
        let root = caller_style(root, self.style_accumulator);
        match self.resizable {
            None => root.layout(root_layout().width(start)),
            Some((min, max)) => {
                let width = signal(start.clamp(min, max.max(min)));
                root.layout(root_layout())
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
    let node = node
        .hover(|s| s.background(Token::SurfaceHover))
        .pressed(|s| s.background(Token::SurfacePressed));
    let node = axis_drag(
        node,
        Orientation::Horizontal,
        move || width.peek(),
        move |base, delta| width.set((base + delta).clamp(min, max)),
        || {},
    );
    resize_keys(
        node,
        (NamedKey::ArrowLeft, NamedKey::ArrowRight),
        move |step| {
            width.set((width.peek() + step).clamp(min, max));
        },
    )
}

builder_impls!([] SidebarBuilder: paint(style_accumulator), widget, node);
