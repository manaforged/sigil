use std::sync::Arc;

use crate::events::CursorIcon;
use crate::events::cursor::node_cursor;
use crate::events::links::{LinkText, byte_at, has_links, link_ranges, link_text};
use crate::geometry::Point;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;

use super::EventRouter;

type Stamp = (NodeId, u64, u64);

#[derive(Default)]
pub(super) struct LinkHover {
    stamp: Option<Stamp>,
    text: Option<LinkText>,
    ranges: Vec<(usize, usize, Arc<str>)>,
}

impl core::fmt::Debug for LinkHover {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LinkHover")
            .field("stamp", &self.stamp)
            .finish_non_exhaustive()
    }
}

impl LinkHover {
    fn over_link(&mut self, cx: &Cx, layout: &LayoutEngine, id: NodeId, global: Point) -> bool {
        let stamp = (id, cx.mutation_generation(), layout.generation());
        if self.stamp != Some(stamp) {
            self.stamp = Some(stamp);
            self.text = link_text(cx, id);
            self.ranges = self.text.as_ref().map(link_ranges).unwrap_or_default();
        }
        let Some(byte) = self.text.as_ref().and_then(|text| byte_at(text, global)) else {
            return false;
        };
        self.ranges
            .iter()
            .any(|(start, end, _)| (*start..*end).contains(&byte))
    }
}

impl EventRouter {
    kittest_pub! {
        /// The cursor of the deepest hovered node that sets one, or the default.
        #[doc(hidden)]
        pub fn cursor(&mut self, cx: &Cx, layout: &LayoutEngine) -> CursorIcon {
            let Some(global) = self.last_pointer.as_ref().map(|pointer| pointer.position) else {
                return CursorIcon::Default;
            };
            let links = &mut self.link_hover;
            for target in self.hover_targets.iter().rev() {
                let Some(node) = cx.get(target.id) else {
                    continue;
                };
                if let Some(cursor) = node_cursor(node) {
                    return cursor;
                }
                if has_links(node) && links.over_link(cx, layout, target.id, global) {
                    return CursorIcon::Pointer;
                }
            }
            CursorIcon::Default
        }
    }
}
