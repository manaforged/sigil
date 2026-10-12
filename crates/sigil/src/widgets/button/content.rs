use crate::kernel::{NodeBuilder, container};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{ControlSize, Metrics};
#[cfg(feature = "scene")]
use crate::widgets::icon::icon_svg;
use crate::widgets::spinner::spinner;

use super::ButtonBuilder;
use super::parts::{BUTTON_FRAME, ButtonFill, SOLID_FRAME, label_node};
use crate::widgets::shared::ICON_GLYPH;

const COMPACT_LABEL_HEIGHT: f32 = 32.0;

impl ButtonBuilder {
    #[cfg(feature = "scene")]
    fn leading_icon(&self, svg: std::rc::Rc<str>) -> crate::widgets::icon::IconBuilder {
        icon_svg(svg)
            .tone(self.resolved_tone())
            .px(if self.height < 30.0 { 14.0 } else { 16.0 })
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
        let Some(svg) = self.leading.clone() else {
            return node.child(label);
        };
        node.child(
            container()
                .hittable(false)
                .layout(
                    LayoutStyle::flex_row()
                        .align_items(AlignItems::Center)
                        .gap(Metrics::current().gap(ControlSize::Compact)),
                )
                .child(self.leading_icon(svg))
                .child(label),
        )
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
            ButtonFill::Ghost => node,
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
                    .gap(Metrics::current().gap(ControlSize::Compact)),
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
