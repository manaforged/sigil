use crate::events::Key;
use crate::layout::Orientation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Move {
    Prev,
    Next,
    PageUp,
    PageDown,
    First,
    Last,
    Parent,
    Child,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Binding {
    Axis(Orientation),
    Always,
    Page,
    Hierarchy,
}

const KEY_MAP: [(Key, Binding, Move); 10] = [
    (
        Key::ArrowDown,
        Binding::Axis(Orientation::Vertical),
        Move::Next,
    ),
    (
        Key::ArrowUp,
        Binding::Axis(Orientation::Vertical),
        Move::Prev,
    ),
    (
        Key::ArrowRight,
        Binding::Axis(Orientation::Horizontal),
        Move::Next,
    ),
    (
        Key::ArrowLeft,
        Binding::Axis(Orientation::Horizontal),
        Move::Prev,
    ),
    (Key::PageDown, Binding::Page, Move::PageDown),
    (Key::PageUp, Binding::Page, Move::PageUp),
    (Key::Home, Binding::Always, Move::First),
    (Key::End, Binding::Always, Move::Last),
    (Key::ArrowLeft, Binding::Hierarchy, Move::Parent),
    (Key::ArrowRight, Binding::Hierarchy, Move::Child),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one independent navigation setting"
)]
pub(crate) struct Nav {
    pub(crate) axis: Orientation,
    pub(crate) wrap: bool,
    pub(crate) skip_disabled: bool,
    pub(crate) page: bool,
    pub(crate) hierarchy: bool,
    pub(crate) restart_past_end: bool,
}

pub(crate) const LISTBOX: Nav = Nav {
    axis: Orientation::Vertical,
    wrap: false,
    skip_disabled: true,
    page: false,
    hierarchy: false,
    restart_past_end: true,
};

pub(crate) const LIST: Nav = Nav {
    axis: Orientation::Vertical,
    wrap: false,
    skip_disabled: false,
    page: true,
    hierarchy: false,
    restart_past_end: false,
};

pub(crate) const TREE: Nav = Nav {
    hierarchy: true,
    ..LIST
};

pub(crate) const TAB_LIST: Nav = Nav {
    axis: Orientation::Horizontal,
    wrap: true,
    skip_disabled: true,
    page: false,
    hierarchy: false,
    restart_past_end: false,
};

pub(crate) const RADIO_GROUP: Nav = Nav {
    axis: Orientation::Vertical,
    wrap: true,
    skip_disabled: false,
    page: false,
    hierarchy: false,
    restart_past_end: false,
};

pub(crate) trait RowSource {
    fn len(&self) -> usize;
    fn enabled(&self, _index: usize) -> bool {
        true
    }
    fn page(&self, _from: usize, _forward: bool) -> Option<usize> {
        None
    }
    fn parent(&self, _index: usize) -> Option<usize> {
        None
    }
    fn child(&self, _index: usize) -> Option<usize> {
        None
    }
}

impl RowSource for usize {
    fn len(&self) -> usize {
        *self
    }
}

impl Nav {
    fn binds(self, binding: Binding) -> bool {
        match binding {
            Binding::Axis(axis) => axis == self.axis,
            Binding::Always => true,
            Binding::Page => self.page,
            Binding::Hierarchy => self.hierarchy,
        }
    }

    pub(crate) fn bindings(self) -> impl Iterator<Item = (Key, Move)> {
        KEY_MAP
            .into_iter()
            .filter(move |(_, binding, _)| self.binds(*binding))
            .map(|(key, _, step)| (key, step))
    }

    pub(crate) fn step_for(self, key: Key) -> Option<Move> {
        self.bindings()
            .find(|(bound, _)| *bound == key)
            .map(|(_, step)| step)
    }

    fn usable(self, rows: &dyn RowSource, index: usize) -> bool {
        !self.skip_disabled || rows.enabled(index)
    }

    pub(crate) fn target(self, rows: &dyn RowSource, from: usize, step: Move) -> Option<usize> {
        let len = rows.len();
        match step {
            Move::First => (0..len).find(|&index| self.usable(rows, index)),
            Move::Last => (0..len).rev().find(|&index| self.usable(rows, index)),
            Move::Next | Move::Prev if self.wrap => self.cycle(rows, from, step == Move::Next),
            Move::Next => {
                let start = if from < len {
                    from + 1
                } else if self.restart_past_end {
                    0
                } else {
                    return None;
                };
                (start..len).find(|&index| self.usable(rows, index))
            }
            Move::Prev => (0..from.min(len))
                .rev()
                .find(|&index| self.usable(rows, index)),
            Move::PageUp => rows.page(from, false),
            Move::PageDown => rows.page(from, true),
            Move::Parent => rows.parent(from),
            Move::Child => rows.child(from),
        }
    }

    fn cycle(self, rows: &dyn RowSource, from: usize, forward: bool) -> Option<usize> {
        let len = rows.len();
        if from >= len || !self.usable(rows, from) {
            return None;
        }
        (1..=len)
            .map(|offset| {
                if forward {
                    (from + offset) % len
                } else {
                    (from + len - offset) % len
                }
            })
            .find(|&index| self.usable(rows, index))
    }
}
