use std::cell::Cell;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::{Key, NodeEventsExt};
use crate::kernel::{
    AccessibleName, Cx, FocusScopeKind, IntoNode, Live, NodeBuilder, Role, TextContent, TopLayer,
    container, queue_announcement,
};
use crate::layout::ViewportOverlay;
use crate::layout::snap;
use crate::layout::{
    AlignItems, Dimension, JustifyContent, LayoutBuilderExt, LayoutStyle, Overflow, Position,
};
use crate::reactive::{Signal, effect_named, signal};
use crate::style::{
    DisplayKind, PaintTarget, Style, StyleAtom, StyleSlot, Token, TransitionId, space,
};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::popup::{self, footer};
use super::super::scroll::contain_scroll;
use super::escape::{close_top_escape_modal, register_escape_modal};
use super::parts::{BODY_GUTTER, CloseControl, dialog_body, dialog_header};

const MODAL_TRANSITION: TransitionId = presets::SMOOTH;

const DIALOG_PADDING: f32 = space(4);

static BACKDROP_BASE: &[StyleAtom] = &[Token::Scrim.bg(), StyleAtom::Transition(MODAL_TRANSITION)];

static DIALOG_BASE: &[StyleAtom] = &[StyleAtom::Transition(MODAL_TRANSITION)];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    Left,
    Right,
    Bottom,
}

pub struct ModalBuilder {
    show: Signal<bool>,
    title: Option<TextContent>,
    children: Vec<Box<dyn IntoNode>>,
    actions: Vec<Box<dyn IntoNode>>,
    style_accumulator: Style,
    trap: bool,
    escape_closes: bool,
    dismiss_on_backdrop: bool,
    close_button: CloseControl,
    side: Option<Side>,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn modal(show: Signal<bool>) -> ModalBuilder {
    ModalBuilder::new(show)
}

#[must_use]
pub fn sheet(show: Signal<bool>) -> ModalBuilder {
    ModalBuilder::new(show)
        .side(Side::Right)
        .dismiss_on_backdrop(true)
}

impl ModalBuilder {
    #[must_use]
    pub fn new(show: Signal<bool>) -> Self {
        Self {
            show,
            title: None,
            children: Vec::new(),
            actions: Vec::new(),
            style_accumulator: Style::new(),
            trap: true,
            escape_closes: true,
            dismiss_on_backdrop: false,
            close_button: CloseControl::Shown,
            side: None,
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub const fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<TextContent>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub const fn show(mut self, show: Signal<bool>) -> Self {
        self.show = show;
        self
    }

    #[must_use]
    pub const fn trap(mut self, enabled: bool) -> Self {
        self.trap = enabled;
        self
    }

    #[must_use]
    pub const fn escape_closes(mut self, enabled: bool) -> Self {
        self.escape_closes = enabled;
        self
    }

    #[must_use]
    pub const fn dismiss_on_backdrop(mut self, enabled: bool) -> Self {
        self.dismiss_on_backdrop = enabled;
        self
    }

    #[must_use]
    pub const fn close_button(mut self, enabled: bool) -> Self {
        self.close_button = if enabled {
            CloseControl::Shown
        } else {
            CloseControl::Hidden
        };
        self
    }

    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    #[must_use]
    pub fn footer(mut self, action: impl IntoNode + 'static) -> Self {
        self.actions.push(Box::new(action));
        self
    }

    #[must_use]
    pub fn actions(
        self,
        cancel: impl IntoNode + 'static,
        confirm: impl IntoNode + 'static,
    ) -> Self {
        self.footer(cancel).footer(confirm)
    }

    fn into_node_builder(self) -> NodeBuilder {
        let show = self.show;
        let root_name = accessible_name(self.title.as_ref());
        let body_scroll = signal(0.0_f32);

        install_open_effects(show, self.title.clone(), body_scroll);
        if self.escape_closes {
            register_escape_modal(show);
        }

        let backdrop = build_backdrop(show, self.dismiss_on_backdrop);
        let dialog = build_dialog(DialogSpec {
            show,
            title: self.title,
            children: self.children,
            actions: self.actions,
            body_scroll,
            style_accumulator: self.style_accumulator,
            trap: self.trap,
            close_button: self.close_button,
            side: self.side,
            layout_overrides: self.layout_overrides,
        });

        let mut root = contain_scroll(
            container()
                .role(Role::GenericContainer)
                .label(root_name)
                .with(TopLayer)
                .with(ViewportOverlay)
                .layout(root_layout(self.side))
                .style_reactive(move || show_display(show))
                .child(backdrop)
                .child(dialog),
        );
        if self.escape_closes {
            root = root.key(Key::Escape, |ev| {
                if close_top_escape_modal() {
                    ev.stop_propagation();
                }
            });
        }

        root
    }
}

impl WidgetLayoutExt for ModalBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

fn install_open_effects(show: Signal<bool>, title: Option<TextContent>, body_scroll: Signal<f32>) {
    let prev = Rc::new(Cell::new(true));
    effect_named("sigil::modal::open", move || {
        let open_now = show.get();
        let was_open = prev.replace(open_now);
        if open_now && !was_open {
            body_scroll.set(0.0);
            let name = title
                .as_ref()
                .map_or_else(|| String::from("Dialog"), TextContent::resolve);
            queue_announcement(name, Live::Polite);
        }
    });
}

impl PaintTarget for ModalBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl IntoNode for ModalBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn root_layout(side: Option<Side>) -> LayoutStyle {
    let root = match side {
        None | Some(Side::Bottom) => LayoutStyle::flex_column(),
        Some(Side::Left | Side::Right) => LayoutStyle::flex_row(),
    };
    let (align, justify) = match side {
        None => (AlignItems::Center, JustifyContent::Center),
        Some(Side::Left) => (AlignItems::Stretch, JustifyContent::Start),
        Some(Side::Right | Side::Bottom) => (AlignItems::Stretch, JustifyContent::End),
    };
    let pad = if side.is_none() {
        snap::detail(DIALOG_PADDING)
    } else {
        0.0
    };
    root.fill()
        .position(Position::Absolute)
        .inset(0.0)
        .padding(pad)
        .align_items(align)
        .justify_content(justify)
}

fn build_backdrop(show: Signal<bool>, dismiss_on_backdrop: bool) -> NodeBuilder {
    let mut backdrop = container()
        .role(Role::GenericContainer)
        .hittable(true)
        .with(ViewportOverlay)
        .layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(0.0)
                .fill(),
        )
        .class(BACKDROP_BASE)
        .style_reactive(move || show_display(show));
    if dismiss_on_backdrop {
        backdrop = backdrop.pointer_down(move |ev| {
            if show.peek() {
                show.set(false);
                ev.stop_propagation();
            }
        });
    }
    backdrop
}

struct DialogSpec {
    show: Signal<bool>,
    title: Option<TextContent>,
    children: Vec<Box<dyn IntoNode>>,
    actions: Vec<Box<dyn IntoNode>>,
    body_scroll: Signal<f32>,
    style_accumulator: Style,
    trap: bool,
    close_button: CloseControl,
    side: Option<Side>,
    layout_overrides: LayoutOverrides,
}

fn build_dialog(spec: DialogSpec) -> NodeBuilder {
    let DialogSpec {
        show,
        title,
        children,
        actions,
        body_scroll,
        style_accumulator,
        trap,
        close_button,
        side,
        layout_overrides,
    } = spec;
    let title_label = accessible_name(title.as_ref());
    let tab_stop = signal(false);

    let mut dialog = popup::surface(Role::Dialog)
        .label(title_label)
        .focusable(true)
        .mutate(move |node| node.a11y.tab_stop = Some(tab_stop))
        .with(layout_overrides.apply(dialog_layout(side)))
        .class(DIALOG_BASE)
        .style_reactive(move || show_display(show));
    if trap {
        dialog = dialog.mutate(|node| {
            node.a11y.focus_scope = Some(FocusScopeKind::Modal);
        });
    }

    if let Some(title_text) = title {
        dialog = dialog.child(dialog_header(
            title_text,
            (close_button == CloseControl::Shown).then_some(show),
        ));
    }
    dialog = dialog.child(dialog_body(children, body_scroll));
    if !actions.is_empty() {
        dialog = dialog.child(footer(actions, BODY_GUTTER));
    }

    if !style_accumulator.is_empty() {
        dialog = dialog.style(style_accumulator);
    }
    dialog
}

const SHEET_WIDTH: f32 = 360.0;
const SHEET_HEIGHT: f32 = 320.0;

fn dialog_layout(side: Option<Side>) -> LayoutStyle {
    let padding_x = snap::line(DIALOG_PADDING);
    let base = LayoutStyle::flex_column()
        .padding_x(padding_x)
        .padding_right((padding_x - BODY_GUTTER).max(0.0))
        .padding_y(snap::detail(DIALOG_PADDING))
        .overflow_y(snap::CellForm::new(Overflow::Visible, Overflow::Hidden).pick())
        .gap(space(2))
        .align_items(AlignItems::Start)
        .justify_content(JustifyContent::Start)
        .max_size(Dimension::Percent(1.0), Dimension::Percent(1.0))
        .shrink(1.0);
    match side {
        None => base.min_size(420.0, 0.0),
        Some(Side::Left | Side::Right) => base.fill_height().width(SHEET_WIDTH),
        Some(Side::Bottom) => base.fill_width().height(SHEET_HEIGHT),
    }
}

fn accessible_name(title: Option<&TextContent>) -> AccessibleName {
    let Some(title) = title else {
        return AccessibleName::Static(String::from("Dialog"));
    };
    match title {
        TextContent::Static(value) => AccessibleName::Static(value.clone()),
        TextContent::Reactive(value) => AccessibleName::Derived(*value),
        TextContent::Compute(_) => AccessibleName::Static(title.resolve()),
    }
}

fn show_display(show: Signal<bool>) -> &'static [StyleAtom] {
    if show.get() {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
