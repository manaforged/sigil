use crate::kernel::{NodeBuilder, container};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{ControlSize, Metrics};
#[cfg(feature = "scene")]
use crate::widgets::icon::icon_svg;
use crate::widgets::spinner::spinner;

use super::ButtonBuilder;
use super::parts::{BUTTON_FRAME, ButtonFill, SOLID_FRAME, compact_label, label_node};
use crate::widgets::shared::ICON_GLYPH;

impl ButtonBuilder {
    pub(super) fn square_content(&self) -> Box<dyn crate::kernel::IntoNode> {
        #[cfg(feature = "scene")]
        if let Some(svg) = self.leading.clone() {
            let size = if self.height < 30.0 { 14.0 } else { 16.0 };
            return Box::new(
                icon_svg(svg)
                    .tone(self.resolved_tone())
                    .px(size)
                    .on_solid(self.resolved_fill() == ButtonFill::Solid),
            );
        }
        Box::new(
            label_node(
                self.label.clone(),
                self.resolved_tone(),
                self.resolved_fill().label_atoms(),
                false,
            )
            .class(ICON_GLYPH),
        )
    }

    #[cfg(feature = "scene")]
    pub(super) fn label_content(&self, node: NodeBuilder) -> NodeBuilder {
        let label = self.label_text();
        let Some(svg) = self.leading.clone() else {
            return node.child(label);
        };
        let size = if self.height < 30.0 { 14.0 } else { 16.0 };
        node.child(
            container()
                .hittable(false)
                .layout(
                    LayoutStyle::flex_row()
                        .align_items(AlignItems::Center)
                        .gap(Metrics::current().gap(ControlSize::Compact)),
                )
                .child(
                    icon_svg(svg)
                        .tone(self.resolved_tone())
                        .px(size)
                        .on_solid(self.resolved_fill() == ButtonFill::Solid),
                )
                .child(label),
        )
    }

    #[cfg(not(feature = "scene"))]
    pub(super) fn label_content(&self, node: NodeBuilder) -> NodeBuilder {
        node.child(self.label_text())
    }

    pub(super) fn label_text(&self) -> crate::kernel::TextBuilder {
        let compact = compact_label(self.height, self.layout_overrides.height_hint());
        label_node(
            self.label.clone(),
            self.resolved_tone(),
            self.resolved_fill().label_atoms(),
            compact,
        )
    }

    pub(super) fn framed(&self, node: NodeBuilder) -> NodeBuilder {
        match (
            self.resolved_fill().frame(),
            self.resolved_fill() == ButtonFill::Solid,
        ) {
            (false, _) => node,
            (true, true) => node.class(SOLID_FRAME),
            (true, false) => node.class(BUTTON_FRAME),
        }
    }

    pub(super) fn pending_content(&self, pending: Signal<bool>) -> NodeBuilder {
        let (label, tone, atoms) = (
            self.label.clone(),
            self.resolved_tone(),
            self.resolved_fill().label_atoms(),
        );
        let compact = compact_label(self.height, self.layout_overrides.height_hint());
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
