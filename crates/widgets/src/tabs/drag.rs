use std::cell::Cell;

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::NodeBuilder;
use sigil_reactive::Signal;

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

    const fn current(self) -> Option<u16> {
        self.inner.current.get()
    }

    fn set_current(self, visual_index: u16) {
        self.inner.current.set(Some(visual_index));
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

    const fn is_dragging(self) -> bool {
        self.inner.source.get().is_some()
    }

    const fn order(self) -> Option<Signal<Vec<usize>>> {
        self.inner.order.get()
    }

    const fn count(self) -> u16 {
        self.inner.count.get()
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
        if !drag.is_dragging() {
            return;
        }
        let Some(current) = drag.current() else {
            return;
        };
        let tab_width = drag.tab_width();
        let tab_list_x = f32::from(current).mul_add(tab_width, event.position.x);
        let target = visual_index_for(tab_list_x, tab_width, drag.count());
        if target != current {
            apply_swap(drag.order(), current, target, usize::from(drag.count()));
            drag.set_current(target);
        }
    })
    .pointer_up(move |_event| {
        drag.end();
    })
}

fn visual_index_for(x: f32, tab_width: f32, count: u16) -> u16 {
    if count == 0 {
        return 0;
    }
    let max = count.saturating_sub(1);
    let mut bucket: u16 = 0;
    let mut probe: u16 = 1;
    while probe <= max {
        let left = f32::from(probe) * tab_width;
        if x >= left {
            bucket = probe;
        } else {
            break;
        }
        probe = probe.saturating_add(1);
        if probe == u16::MAX {
            break;
        }
    }
    bucket.min(max)
}

fn apply_swap(order: Option<Signal<Vec<usize>>>, from: u16, to: u16, count: usize) {
    let Some(signal) = order else {
        return;
    };
    let mut next = signal.peek();
    if next.len() != count {
        next = (0..count).collect();
    }
    let from_idx = usize::from(from);
    let to_idx = usize::from(to);
    if from_idx < next.len() && to_idx < next.len() {
        next.swap(from_idx, to_idx);
        signal.set(next);
    }
}
