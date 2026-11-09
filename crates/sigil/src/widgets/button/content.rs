use crate::kernel::{NodeBuilder, container};
use crate::layout::Space;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{ControlSize, Metrics};
use crate::widgets::Widget;
#[cfg(feature = "scene")]
use crate::widgets::icon::icon_svg;
use crate::widgets::spinner::spinner;

use super::ButtonBuilder;
use super::parts::{BUTTON_FRAME, ButtonFill, GHOST_FRAME, SOLID_FRAME, label_node};
use crate::widgets::shared::ICON_GLYPH;

const COMPACT_LABEL_HEIGHT: f32 = 32.0;

impl ButtonBuilder {
    #[cfg(feature = "scene")]
    fn leading_icon(&self, svg: std::sync::Arc<str>) -> crate::widgets::icon::IconBuilder {
        icon_svg(svg)
            .tone(self.resolved_tone())
            .px(if self.height < 30.0 { 14.0_f32 } else { 16.0 }.min(self.height))
            .on_solid(self.resolved_fill() == ButtonFill::Solid)
    }

    pub(super) fn square_content(&self) -> Box<dyn crate::kernel::IntoNode> {
        #[cfg(feature = "scene")]
        if let Some(svg) = self.leading.clone() {
            return Box::new(self.leading_icon(svg));
        }
        Box::new(self.label_text(false).class(ICON_GLYPH))
    }

    #[cfg(feature = "scene")]
    pub(super) fn label_content(&self, node: NodeBuilder) -> NodeBuilder {
        let label = self.label_text(self.compact());
        if self.leading.is_none() && self.trailing.is_none() && self.shortcut.is_none() {
            return node.child(label);
        }
        let mut row = container().hittable(false).layout(
            LayoutStyle::flex_row()
                .align_items(AlignItems::Center)
                .gap(Space::px(Metrics::current().gap(ControlSize::Compact))),
        );
        if let Some(svg) = self.leading.clone() {
            row = row.child(self.leading_icon(svg));
        }
        row = row.child(label);
        if let Some(svg) = self.trailing.clone() {
            row = row.child(self.leading_icon(svg).px(12.0));
        }
        if let Some(key) = self.shortcut.clone() {
            let (tone, atoms) = (self.resolved_tone(), self.resolved_fill().label_atoms());
            row = row.child(label_node(key.into(), tone, atoms, true).class("opacity-60"));
        }
        node.child(row)
    }

    #[cfg(not(feature = "scene"))]
    pub(super) fn label_content(&self, node: NodeBuilder) -> NodeBuilder {
        node.child(self.label_text(self.compact()))
    }

    fn compact(&self) -> bool {
        self.height < Metrics::current().control_height(ControlSize::Default)
            || self
                .layout_overrides
                .height_hint()
                .is_some_and(|px| px <= COMPACT_LABEL_HEIGHT)
    }

    fn label_text(&self, compact: bool) -> crate::kernel::TextBuilder {
        let (tone, atoms) = (self.resolved_tone(), self.resolved_fill().label_atoms());
        label_node(self.label.clone(), tone, atoms, compact)
    }

    pub(super) fn framed(&self, node: NodeBuilder) -> NodeBuilder {
        match self.resolved_fill() {
            ButtonFill::Ghost => node.class(GHOST_FRAME),
            ButtonFill::Solid => node.class(SOLID_FRAME),
            ButtonFill::Soft | ButtonFill::Outline => node.class(BUTTON_FRAME),
        }
    }

    pub(super) fn pending_content(&self, pending: Signal<bool>) -> NodeBuilder {
        let (label, tone, atoms) = (
            self.label.clone(),
            self.resolved_tone(),
            self.resolved_fill().label_atoms(),
        );
        let compact = self.compact();
        container()
            .hittable(false)
            .layout(
                LayoutStyle::flex_row()
                    .align_items(AlignItems::Center)
                    .gap(Space::px(Metrics::current().gap(ControlSize::Compact))),
            )
            .class(atoms)
            .children_reactive(move || {
                let mut parts = Vec::with_capacity(2);
                if pending.get() {
                    parts.push(spinner().hittable(false));
                }
                parts.push(container().child(label_node(label.clone(), tone, atoms, compact)));
                parts
            })
    }
}
