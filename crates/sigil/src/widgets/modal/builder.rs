use crate::style::strings::StringKey;
use crate::style::theme::ambient::ActiveTheme;

use crate::animation_runtime::presets;
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TextContent, container};
use crate::layout::snap;
use crate::layout::{AlignItems, Dimension, JustifyContent, LayoutStyle, Overflow, Position};
use crate::reactive::{Signal, signal};
use crate::style::{Style, StyleAtom, Token, TransitionId, space};
use crate::widgets::Widget;

use super::super::layout_ext::LayoutOverrides;
use super::super::popup::{self, PopupAnchor, PopupLayer, close_on_escape, footer};
use super::super::scroll::contain_scroll;
use super::super::shared::accessible_name_or;
use super::parts::{BODY_GUTTER, CloseControl, dialog_body, dialog_header};

const MODAL_TRANSITION: TransitionId = presets::SMOOTH;
const DIALOG_PADDING: f32 = space(4);
const SHEET_WIDTH: f32 = 360.0;
const SHEET_HEIGHT: f32 = 320.0;

static BACKDROP_BASE: &[StyleAtom] = &[Token::Scrim.bg(), StyleAtom::Transition(MODAL_TRANSITION)];

static DIALOG_BASE: &[StyleAtom] = &[StyleAtom::Transition(MODAL_TRANSITION)];

/// Edge of a sheet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    /// The left side.
    Left,
    /// The right side.
    Right,
    /// The bottom side.
    Bottom,
}

/// Builder of a modal or sheet.
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

/// A modal dialog.
#[must_use]
pub fn modal(show: Signal<bool>) -> ModalBuilder {
    ModalBuilder::new(show)
}

/// A panel that slides in from a side.
#[must_use]
pub fn sheet(show: Signal<bool>) -> ModalBuilder {
    ModalBuilder::new(show)
        .side(Side::Right)
        .dismiss_on_backdrop(true)
}

impl ModalBuilder {
    /// Creates a modal builder.
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

    /// Sets the side.
    #[must_use]
    pub const fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    /// Sets the title.
    #[must_use]
    pub fn title(mut self, title: impl Into<TextContent>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the show.
    #[must_use]
    pub const fn show(mut self, show: Signal<bool>) -> Self {
        self.show = show;
        self
    }

    /// Sets the trap.
    #[must_use]
    pub const fn trap(mut self, enabled: bool) -> Self {
        self.trap = enabled;
        self
    }

    /// Sets the escape closes.
    #[must_use]
    pub const fn escape_closes(mut self, enabled: bool) -> Self {
        self.escape_closes = enabled;
        self
    }

    /// Sets the dismiss on backdrop.
    #[must_use]
    pub const fn dismiss_on_backdrop(mut self, enabled: bool) -> Self {
        self.dismiss_on_backdrop = enabled;
        self
    }

    /// Sets the close button.
    #[must_use]
    pub const fn close_button(mut self, enabled: bool) -> Self {
        self.close_button = if enabled {
            CloseControl::Shown
        } else {
            CloseControl::Hidden
        };
        self
    }

    /// Adds a child.
    #[must_use]
    pub fn child(mut self, child: impl IntoNode + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Sets the footer.
    #[must_use]
    pub fn footer(mut self, action: impl IntoNode + 'static) -> Self {
        self.actions.push(Box::new(action));
        self
    }

    /// Sets the actions.
    #[must_use]
    pub fn actions(
        self,
        cancel: impl IntoNode + 'static,
        confirm: impl IntoNode + 'static,
    ) -> Self {
        self.footer(cancel).footer(confirm)
    }

    fn root(self) -> NodeBuilder {
        let show = self.show;
        let root_name = accessible_name_or(self.title.as_ref(), StringKey::Dialog);
        let body_scroll = signal(0.0_f32);
        let layer = PopupLayer::new(show, PopupAnchor::Top);

        install_open_effects(layer, self.title.clone(), body_scroll);
        if self.escape_closes {
            layer.register_escape();
        }

        let side = self.side;
        let escape_closes = self.escape_closes;
        let backdrop = build_backdrop(show, self.dismiss_on_backdrop);
        let dialog = build_dialog(self, body_scroll);

        let root = contain_scroll(
            layer
                .wrap(
                    container()
                        .role(Role::GenericContainer)
                        .label(root_name)
                        .layout(root_layout(side)),
                )
                .child(backdrop)
                .child(dialog),
        );
        if escape_closes {
            close_on_escape(root)
        } else {
            root
        }
    }
}

builder_impls!([] ModalBuilder: paint(style_accumulator), widget);

impl IntoNode for ModalBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

fn install_open_effects(layer: PopupLayer, title: Option<TextContent>, body_scroll: Signal<f32>) {
    let theme = ActiveTheme::capture();
    layer.announce_open("sigil::modal::open", move || {
        body_scroll.set(0.0);
        title
            .as_ref()
            .map_or_else(|| theme.string(StringKey::Dialog), TextContent::resolve)
    });
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
    let layer = PopupLayer::new(show, PopupAnchor::Viewport);
    let backdrop = layer.wrap(
        container()
            .role(Role::GenericContainer)
            .hittable(true)
            .layout(
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_top(0.0)
                    .inset_left(0.0)
                    .fill(),
            )
            .class(BACKDROP_BASE),
    );
    if dismiss_on_backdrop {
        layer.dismiss_outside(backdrop)
    } else {
        backdrop
    }
}

fn build_dialog(spec: ModalBuilder, body_scroll: Signal<f32>) -> NodeBuilder {
    let ModalBuilder {
        show,
        title,
        children,
        actions,
        style_accumulator,
        trap,
        close_button,
        side,
        layout_overrides,
        ..
    } = spec;
    let title_label = accessible_name_or(title.as_ref(), StringKey::Dialog);
    let tab_stop = signal(false);

    let mut dialog = PopupLayer::new(show, PopupAnchor::Inline).trap(trap).wrap(
        popup::surface(Role::Dialog)
            .label(title_label)
            .focusable(true)
            .mutate(move |node| node.a11y.tab_stop = Some(tab_stop))
            .with(dialog_layout(side))
            .class(DIALOG_BASE),
    );

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
    layout_overrides.finish(dialog)
}

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

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
