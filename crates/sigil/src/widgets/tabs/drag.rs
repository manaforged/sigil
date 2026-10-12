use std::cell::Cell;

use crate::events::NodeEventsExt;
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;

#[derive(Clone, Copy)]
pub(super) struct DragState {
    inner: &'static DragStateInner,
}

struct DragStateInner {
    source: Cell<Option<u16>>,
    current: Cell<Option<u16>>,
    just_dragged: Cell<bool>,
    order: Cell<Option<Signal<Vec<usize>>>>,
    count: Cell<u16>,
    tab_width: Cell<f32>,
}

impl DragState {
    pub(super) fn new() -> Self {
        let inner = Box::leak(Box::new(DragStateInner {
            source: Cell::new(None),
            current: Cell::new(None),
            just_dragged: Cell::new(false),
            order: Cell::new(None),
            count: Cell::new(0),
            tab_width: Cell::new(0.0),
        }));
        Self { inner }
    }

    pub(super) fn bind(self, order: Option<Signal<Vec<usize>>>, count: usize) {
        self.inner.order.set(order);
        self.inner
            .count
            .set(u16::try_from(count).unwrap_or(u16::MAX));
    }

    fn begin(self, visual_index: u16, width: f32) {
        self.inner.source.set(Some(visual_index));
        self.inner.current.set(Some(visual_index));
        self.inner.just_dragged.set(false);
        self.inner.tab_width.set(width);
    }

    fn tab_width(self) -> f32 {
        let width = self.inner.tab_width.get();
        if width > 1.0 { width } else { 80.0 }
    }

    fn end(self) {
        let source = self.inner.source.take();
        let current = self.inner.current.take();
        if let (Some(start), Some(now)) = (source, current)
            && start != now
        {
            self.inner.just_dragged.set(true);
        }
    }

    pub(super) const fn consume_just_dragged(self) -> bool {
        self.inner.just_dragged.replace(false)
    }
}

pub(super) fn install_label_drag_hooks(
    node: NodeBuilder,
    drag: DragState,
    visual_index: usize,
) -> NodeBuilder {
    let slot = u16::try_from(visual_index).unwrap_or(u16::MAX);
    node.pointer_down(move |event| {
        drag.begin(slot, event.target_size.width);
        event.capture_pointer();
    })
    .pointer_move(move |event| {
        let inner = drag.inner;
        let (Some(_), Some(current)) = (inner.source.get(), inner.current.get()) else {
            return;
        };
        let tab_width = drag.tab_width();
        let tab_list_x = f32::from(current).mul_add(tab_width, event.position.x);
        let target = (1..inner.count.get())
            .take_while(|&probe| tab_list_x >= f32::from(probe) * tab_width)
            .last()
            .unwrap_or(0);
        if target != current {
            apply_swap(
                inner.order.get(),
                current,
                target,
                usize::from(inner.count.get()),
            );
            inner.current.set(Some(target));
        }
    })
    .pointer_up(move |_| drag.end())
}

fn apply_swap(order: Option<Signal<Vec<usize>>>, from: u16, to: u16, count: usize) {
    let Some(signal) = order else {
        return;
    };
    let mut next = signal.peek();
    if next.len() != count {
        next = (0..count).collect();
    }
    let (from, to) = (usize::from(from), usize::from(to));
    if from < next.len() && to < next.len() {
        next.swap(from, to);
        signal.set(next);
    }
}
