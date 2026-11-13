use crate::layout::{AlignItems, LayoutStyle, Length, Space};
use crate::node::{IntoNode, NodeBuilder, container};
use crate::reactive::Signal;
use crate::style::{ControlSize, Metrics, Paint};
#[cfg(feature = "scene")]
use crate::widgets::{Glyph, IconBuilder, icon};
use crate::widgets::{Widget, spinner};

use super::ButtonBuilder;
#[cfg(feature = "scene")]
use super::parts::ButtonFill;
use super::parts::{apply_label_paint, label_node};

const COMPACT_LABEL_HEIGHT: f32 = 32.0;

impl ButtonBuilder {
    #[cfg(feature = "scene")]
    fn leading_icon(&self, svg: std::sync::Arc<str>) -> IconBuilder {
        let metrics = Metrics::current();
        let px = if metrics.is_grid() {
            metrics.icon_size()
        } else if self.height < 30.0 {
            14.0
        } else {
            16.0
        };
        icon(Glyph::svg(svg))
            .tone(self.resolved_tone())
            .px(px.min(self.height))
            .on_solid(self.resolved_fill() == ButtonFill::Solid)
    }

    pub(super) fn square_content(&self) -> Box<dyn IntoNode> {
        #[cfg(feature = "scene")]
        if let Some(svg) = self.leading.clone() {
            return Box::new(self.leading_icon(svg));
        }
        Box::new(
            self.label_text(false)
                .font_size(Metrics::current().icon_size(), "square button glyph"),
        )
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
                .gap(Space::raw(
                    Metrics::current().control_gap(ControlSize::Compact),
                    "control gap",
                )),
        );
        if let Some(svg) = self.leading.clone() {
            row = row.child(self.leading_icon(svg));
        }
        row = row.child(label);
        if let Some(svg) = self.trailing.clone() {
            row = row.child(self.leading_icon(svg).px(12.0));
        }
        if let Some(key) = self.shortcut.clone() {
            let (tone, fill) = (self.resolved_tone(), self.resolved_fill());
            row = row.child(label_node(key.into(), tone, fill, true).opacity(0.6));
        }
        node.child(row)
    }

    #[cfg(not(feature = "scene"))]
    pub(super) fn label_content(&self, node: NodeBuilder) -> NodeBuilder {
        node.child(self.label_text(self.compact()))
    }

    fn compact(&self) -> bool {
        self.height < Metrics::current().control_height(ControlSize::Default)
            || matches!(self.frame.height(), Some(Length::Px(px)) if px <= COMPACT_LABEL_HEIGHT)
    }

    fn label_text(&self, compact: bool) -> crate::node::TextBuilder {
        label_node(
            self.label.clone(),
            self.resolved_tone(),
            self.resolved_fill(),
            compact,
        )
    }

    pub(super) fn pending_content(&self, pending: Signal<bool>) -> NodeBuilder {
        let (label, tone, fill) = (
            self.label.clone(),
            self.resolved_tone(),
            self.resolved_fill(),
        );
        let compact = self.compact();
        let row = container().hittable(false).layout(
            LayoutStyle::flex_row()
                .align_items(AlignItems::Center)
                .gap(Space::raw(
                    Metrics::current().control_gap(ControlSize::Compact),
                    "control gap",
                )),
        );
        apply_label_paint(row, fill, tone).children_reactive(move || {
            let mut parts = Vec::with_capacity(2);
            if pending.get() {
                parts.push(spinner().hittable(false));
            }
            parts.push(container().child(label_node(label.clone(), tone, fill, compact)));
            parts
        })
    }
}
