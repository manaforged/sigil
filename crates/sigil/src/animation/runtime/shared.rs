use std::cell::RefCell;
use std::hash::{Hash, Hasher};

use ahash::{AHashMap, AHasher};

use crate::animation::runtime::layout_motion::{
    CHANNELS, Deltas, Glide, RADIUS, apply, glide_of, peek,
};
use crate::animation::runtime::style::theme_at;
use crate::geometry::Rect;
use crate::kernel::{Cx, Node, NodeBuilder, NodeId};
use crate::reactive::{Signal, cleanup, owner_by_id};
use crate::style::{StyleBinding, render_style_for};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Shared(u64);

impl Shared {
    pub(crate) fn of(key: &impl Hash) -> Self {
        let mut hasher = AHasher::default();
        key.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Visual {
    pub(crate) rect: Rect,
    pub(crate) radius: f32,
}

struct Last {
    node: NodeId,
    rect: Rect,
    radius: f32,
    deltas: Option<[Signal<f32>; CHANNELS]>,
    frozen: Option<Deltas>,
}

impl Last {
    fn live(&self) -> Deltas {
        self.deltas.map(peek).unwrap_or_default()
    }

    fn visual(&self) -> Visual {
        let deltas = self.frozen.unwrap_or_else(|| self.live());
        Visual {
            rect: apply(self.rect, deltas),
            radius: (self.radius + deltas[RADIUS]).max(0.0),
        }
    }
}

type Slot = (u64, u64);

thread_local! {
    static LAST: RefCell<AHashMap<Slot, Last>> = RefCell::new(AHashMap::new());
}

pub(crate) fn attach(node: NodeBuilder, key: Shared) -> NodeBuilder {
    node.mutate(move |target| {
        if target.extension::<StyleBinding>().is_none() {
            let _ = target.attach(StyleBinding::default());
        }
        let _ = target.attach(key);
    })
}

pub(crate) fn start(cx: &Cx, key: Shared, id: NodeId) -> Option<Visual> {
    LAST.with_borrow(|last| {
        last.get(&(cx.instance_id(), key.0))
            .filter(|entry| entry.node != id)
            .map(Last::visual)
    })
}

pub(crate) fn note(cx: &Cx, key: Shared, id: NodeId, screen: Rect, arrived: bool) {
    let Some(node) = cx.get(id) else {
        return;
    };
    let slot = (cx.instance_id(), key.0);
    let entry = Last {
        node: id,
        rect: screen,
        radius: rest_radius(cx, id, node),
        deltas: glide_of(node).map(Glide::deltas),
        frozen: None,
    };
    let claimed = LAST.with_borrow_mut(|last| {
        let held = last.get(&slot).map(|kept| kept.node);
        let claim = held != Some(id) && (held.is_none() || arrived);
        if claim || held == Some(id) {
            last.insert(slot, entry);
        }
        claim
    });
    if claimed {
        owner_by_id(node.owner, || cleanup(move || retire(slot, id)));
    }
}

fn retire(slot: Slot, id: NodeId) {
    let _ = LAST.try_with(|cell| {
        let mut last = cell.borrow_mut();
        if let Some(entry) = last.get_mut(&slot).filter(|entry| entry.node == id) {
            entry.frozen = Some(entry.live());
        }
    });
}

pub(crate) fn settle(cx: &Cx) {
    let instance = cx.instance_id();
    let _ = LAST.try_with(|cell| {
        let mut last = cell.borrow_mut();
        if last.is_empty() {
            return;
        }
        last.retain(|(at, _), entry| {
            *at != instance || (entry.frozen.is_none() && cx.get(entry.node).is_some())
        });
    });
}

pub(crate) fn rest_radius(cx: &Cx, id: NodeId, node: &Node) -> f32 {
    glide_of(node)
        .and_then(Glide::rest_radius)
        .unwrap_or_else(|| {
            render_style_for(node, theme_at(cx, id), None)
                .border_radius
                .map_or(0.0, f32::from)
        })
}
