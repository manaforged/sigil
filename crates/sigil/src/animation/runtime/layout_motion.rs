use std::cell::{Cell, RefCell};

use slotmap::{DefaultKey, SecondaryMap};
use smallvec::SmallVec;

use crate::animation::runtime::registry::{glides, morph_config};
use crate::animation::runtime::shared::{self, Shared};
use crate::animation::runtime::style::record_on;
use crate::animation::runtime::{AnimationConfig, MotionPrefs, SignalAnimator};
use crate::geometry::{Point, Rect};
use crate::kernel::{Cx, DeltaKind, Node, NodeId, NodeTag, NodeType};
use crate::layout::LayoutResult;
use crate::reactive::{Signal, batch, owner_by_id, signal};
use crate::style::{NumericAtomKind, StyleAtom, StyleBinding};

pub(crate) const CHANNELS: usize = 5;

const WIDTH: usize = 2;

const HEIGHT: usize = 3;

pub(crate) const RADIUS: usize = 4;

pub(crate) type Deltas = [f32; CHANNELS];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    place: Rect,
    screen: Rect,
}

impl Placed {
    pub(crate) fn of(
        cx: &Cx,
        results: &SecondaryMap<DefaultKey, LayoutResult>,
        id: NodeId,
    ) -> Option<Self> {
        let screen = results.get(id.key())?.rect();
        let base = std::iter::successors(cx.parent(id), |&at| cx.parent(at))
            .find_map(|at| results.get(at.key()))
            .map_or(Point::new(0.0, 0.0), |parent| parent.rect().origin);
        let origin = Point::new(screen.origin.x - base.x, screen.origin.y - base.y);
        Some(Self {
            place: Rect::new(origin, screen.width, screen.height),
            screen,
        })
    }
}

pub(crate) struct Glide {
    deltas: [Signal<f32>; CHANNELS],
    base_radius: Cell<f32>,
    animators: RefCell<[SignalAnimator; CHANNELS]>,
}

impl Glide {
    fn new(tag: Option<NodeTag>, config: AnimationConfig) -> Self {
        let deltas: [Signal<f32>; CHANNELS] = std::array::from_fn(|_| signal(0.0_f32));
        record_on(tag, &deltas, DeltaKind::Paint);
        Self {
            deltas,
            base_radius: Cell::new(0.0),
            animators: RefCell::new(deltas.map(|value| SignalAnimator::new(value, config))),
        }
    }

    fn now(&self) -> Deltas {
        peek(self.deltas)
    }

    pub(crate) fn offset(&self) -> Option<[f32; 2]> {
        let [x, y, ..] = self.now();
        (moving(x) || moving(y)).then_some([x, y])
    }

    pub(crate) fn size(&self) -> Option<[f32; 2]> {
        let [_, _, width, height, _] = self.now();
        (moving(width) || moving(height)).then_some([width, height])
    }

    pub(crate) fn radius(&self) -> Option<f32> {
        let delta = self.deltas[RADIUS].try_peek()?;
        moving(delta).then(|| (self.base_radius.get() + delta).max(0.0))
    }

    pub(crate) fn value_for(&self, kind: NumericAtomKind) -> Option<f32> {
        match kind {
            NumericAtomKind::BorderRadius => self.radius(),
            _ => None,
        }
    }

    pub(crate) fn bits(&self) -> Option<[u32; CHANNELS]> {
        let now = self.now();
        now.into_iter().any(moving).then(|| now.map(f32::to_bits))
    }

    pub(crate) const fn deltas(&self) -> [Signal<f32>; CHANNELS] {
        self.deltas
    }

    pub(crate) fn rest_radius(&self) -> Option<f32> {
        self.radius().map(|_| self.base_radius.get())
    }

    fn shift(&self, by: Deltas, config: AnimationConfig) {
        batch(|| {
            let mut animators = self.animators.borrow_mut();
            for (animator, step) in animators.iter_mut().zip(by) {
                if !moving(step) {
                    continue;
                }
                animator.set_config(config);
                animator.rebase(step);
                animator.target(0.0);
            }
        });
    }

    fn rest(&self) {
        batch(|| {
            for animator in self.animators.borrow_mut().iter_mut() {
                animator.snap(0.0);
            }
        });
    }
}

const fn moving(value: f32) -> bool {
    value.abs() > f32::EPSILON
}

pub(crate) fn peek(deltas: [Signal<f32>; CHANNELS]) -> Deltas {
    deltas.map(|value| value.try_peek().unwrap_or_default())
}

pub(crate) const fn apply(rect: Rect, [x, y, width, height, _]: Deltas) -> Rect {
    Rect::new(
        Point::new(rect.origin.x + x, rect.origin.y + y),
        (rect.width + width).max(0.0),
        (rect.height + height).max(0.0),
    )
}

pub(crate) fn opted_in(node: &Node) -> bool {
    node.extension::<Shared>().is_some()
        || node
            .extension::<StyleBinding>()
            .filter(|binding| binding.has_transition())
            .is_some_and(|binding| glides(binding.cascade()))
}

pub(crate) fn glide(cx: &Cx, id: NodeId, from: Option<Placed>, to: Placed) {
    let Some(node) = cx.get(id) else {
        return;
    };
    let key = node.extension::<Shared>().copied();
    if let Some(from) = from {
        morph(node, moved(from.place, to.place), None, key.is_some());
    } else if let Some((by, radius)) = key.and_then(|key| arrival(cx, (id, node), key, to.screen)) {
        morph(node, by, Some(radius), true);
    }
    if let Some(key) = key {
        shared::note(cx, key, id, to.screen, from.is_none());
    }
}

const fn moved(from: Rect, to: Rect) -> Deltas {
    [
        from.origin.x - to.origin.x,
        from.origin.y - to.origin.y,
        from.width - to.width,
        from.height - to.height,
        0.0,
    ]
}

fn arrival(cx: &Cx, (id, node): (NodeId, &Node), key: Shared, to: Rect) -> Option<(Deltas, f32)> {
    let start = shared::start(cx, key, id)?;
    let radius = shared::rest_radius(cx, id, node);
    let mut by = moved(start.rect, to);
    by[RADIUS] = start.radius - radius;
    Some((by, radius))
}

fn morph(node: &Node, mut by: Deltas, radius: Option<f32>, shared: bool) {
    if matches!(node.kind, NodeType::Text { .. }) {
        by[WIDTH] = 0.0;
        by[HEIGHT] = 0.0;
    }
    if !by.into_iter().any(moving) {
        return;
    }
    let Some(binding) = node.extension::<StyleBinding>() else {
        return;
    };
    let cell = binding.glide();
    if MotionPrefs::reduced() {
        if let Some(glide) = cell.get() {
            glide.rest();
        }
        return;
    }
    let atoms: SmallVec<[StyleAtom; 32]> = binding.cascade().collect();
    let Some(config) = morph_config(&atoms, shared) else {
        return;
    };
    if cell.get().is_none() {
        owner_by_id(node.owner, || {
            let _ = cell.set(Box::new(Glide::new(node.tag(), config)));
        });
    }
    if let Some(glide) = cell.get() {
        if let Some(radius) = radius {
            glide.base_radius.set(radius);
        }
        glide.shift(by, config);
    }
}

pub(crate) fn box_rect(node: &Node, rect: Rect) -> Option<Rect> {
    if matches!(node.kind, NodeType::Text { .. } | NodeType::Fragment) {
        return None;
    }
    let [width, height] = glide_of(node)?.size()?;
    Some(apply(rect, [0.0, 0.0, width, height, 0.0]))
}

pub(crate) fn glide_of(node: &Node) -> Option<&Glide> {
    node.extension::<StyleBinding>()?
        .glide()
        .get()
        .map(Box::as_ref)
}

pub(crate) fn paint_bounds(node: &Node, rect: Rect) -> Rect {
    box_rect(node, rect).map_or(rect, |grown| grown.union(rect))
}
