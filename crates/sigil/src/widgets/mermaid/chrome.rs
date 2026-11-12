use crate::layout::Space;
use std::time::Duration;

use crate::icons::Icon;
use crate::kernel::{AccessibleName, NodeBuilder, Role, container, text};
use crate::layout::{AlignItems, LayoutStyle, Position};
use crate::reactive::{Signal, memo, signal, timeout};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_string_once};
use crate::style::{ControlSize, Paint, Radius, Token, space};
use crate::widgets::{ButtonBuilder, button};
use crate::widgets::{Layout, Widget};

use super::{Drawn, diagram_name, fit, lightbox, scrolled};

const COPIED_FOR: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy)]
pub(super) struct Chrome {
    code: Signal<bool>,
    open: Signal<bool>,
    hover: Signal<bool>,
    focus: Signal<bool>,
    copied: Signal<bool>,
}

impl Chrome {
    pub(super) fn new() -> Self {
        Self {
            code: signal(false),
            open: signal(false),
            hover: signal(false),
            focus: signal(false),
            copied: signal(false),
        }
    }

    pub(super) fn wrap(self, drawn: Drawn, source: &str, stale: bool) -> NodeBuilder {
        let name = diagram_name(source);
        let expandable = drawn.height.is_some();
        let lightbox = expandable.then(|| lightbox::lightbox(self.open, &drawn.source, &name));
        let mut root = container()
            .role(Role::Figure)
            .label(name)
            .description(source.trim_end().to_string())
            .layout(LayoutStyle::flex_column().fill_width())
            .hittable(true)
            .pointer_enter(move |_| self.hover.set(true))
            .pointer_leave(move |_| self.hover.set(false))
            .focus_in(move || self.focus.set(true))
            .focus_out(move || self.focus.set(false))
            .child(
                self.body(drawn, source)
                    .paint_when(stale, |s| s.opacity(0.6)),
            )
            .child(self.toolbar(source, expandable));
        if stale {
            root = root.child(updating());
        }
        if let Some(lightbox) = lightbox {
            root = root.child(lightbox);
        }
        root
    }

    fn body(self, drawn: Drawn, source: &str) -> NodeBuilder {
        let code = self.code;
        let preview = match drawn.height {
            Some(height) => fit::fitted(drawn.node, (drawn.width, height)),
            None => scrolled(drawn.node, drawn.width),
        };
        container()
            .layout(LayoutStyle::flex_column().fill_width())
            .child(preview.visible_when(move || !code.get()))
            .child(code_view(source).visible_when(code))
    }

    fn toolbar(self, source: &str, expandable: bool) -> NodeBuilder {
        let (hover, focus, copied) = (self.hover, self.focus, self.copied);
        let hidden = memo(move || !(hover.get() || focus.get() || copied.get()));
        let mut bar = container()
            .layout(
                LayoutStyle::flex_row()
                    .position(Position::Absolute)
                    .inset_top(space(1))
                    .inset_right(space(1))
                    .gap(Space::Xs)
                    .padding(Space::Xs)
                    .align_items(AlignItems::Center),
            )
            .background(Token::Surface)
            .border_color(Token::Border)
            .border_width(1)
            .radius(Radius::Md)
            .paint_when(hidden, |s| s.opacity(0.0))
            .child(self.toggle())
            .child(self.copy(source));
        if expandable {
            let open = self.open;
            bar = bar.child(
                tool(Icon::Maximize2, move || open.set(true))
                    .accessible(theme_string_once(StringKey::ExpandDiagram)),
            );
        }
        bar
    }

    fn toggle(self) -> ButtonBuilder {
        let code = self.code;
        let theme = ActiveTheme::capture();
        let name = memo(move || {
            theme.string(if code.get() {
                StringKey::ShowDiagram
            } else {
                StringKey::ShowCode
            })
        });
        tool(Icon::Code, move || code.update(|on| *on = !*on))
            .accessible(AccessibleName::Derived(name))
            .paint_when(code, |s| {
                s.background(Token::Accent3).text_color(Token::Accent11)
            })
    }

    fn copy(self, source: &str) -> ButtonBuilder {
        let copied = self.copied;
        let body = source.trim_end().to_string();
        let theme = ActiveTheme::capture();
        let name = memo(move || {
            theme.string(if copied.get() {
                StringKey::Copied
            } else {
                StringKey::CopySource
            })
        });
        tool(Icon::Copy, move || {
            if crate::clipboard::copy(&body).is_ok() {
                copied.set(true);
                let _ = timeout(COPIED_FOR, move || copied.set(false));
            }
        })
        .accessible(AccessibleName::Derived(name))
        .paint_when(copied, |s| s.text_color(Token::Success11))
    }
}

fn tool(icon: Icon, activate: impl Fn() + 'static) -> ButtonBuilder {
    button(icon).activate(activate).size(ControlSize::Compact)
}

fn code_view(source: &str) -> NodeBuilder {
    let mut block = container()
        .role(Role::Code)
        .class("font-mono text-sm")
        .layout(LayoutStyle::flex_column().fill_width().padding(Space::Md))
        .background(Token::Field)
        .radius(Radius::Md)
        .selectable_text(true);
    for line in source.trim_end().lines() {
        let line = if line.is_empty() { " " } else { line };
        block = block.child(text(line.to_string()).class(Token::OnSurface.text()));
    }
    block
}

fn updating() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_top(space(1))
                .inset_left(space(1))
                .padding_x(Space::Sm)
                .padding_y(Space::Xs),
        )
        .background(Token::SurfaceSubtle)
        .radius(Radius::Md)
        .child(
            text("Updating…")
                .class("text-xs")
                .class(Token::OnSurfaceMuted.text()),
        )
}
