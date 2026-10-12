use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, container};
use crate::layout::LayoutReactiveExt;
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived, effect, signal};

use super::render::WidthSpec;
use super::{RowKey, bound_slice};

pub(super) type Tops = Signal<Rc<Vec<f32>>>;
pub(super) type Keys = Signal<Rc<Vec<u64>>>;

type Cache = Rc<RefCell<HashMap<u64, f32>>>;

#[derive(Clone)]
pub(super) struct Measure {
    cache: Cache,
    width: Rc<Cell<f32>>,
    version: Signal<u64>,
    pub(super) tops: Tops,
    pub(super) keys: Option<Keys>,
}

pub(super) fn row_key_of<T>(row_key: Option<&RowKey<T>>, index: usize, item: &T) -> u64 {
    row_key.map_or(index as u64, |key_of| key_of(index, item))
}

fn prune<S, T>(items: Signal<S>, row_key: Option<RowKey<T>>, cache: Cache)
where
    S: Deref + Clone + 'static,
    S::Target: AsRef<[T]>,
    T: 'static,
{
    effect(move || {
        let items = items.get();
        let live: HashSet<u64> = bound_slice(&items)
            .iter()
            .enumerate()
            .map(|(index, item)| row_key_of(row_key.as_ref(), index, item))
            .collect();
        cache.borrow_mut().retain(|key, _| live.contains(key));
    });
}

impl Measure {
    pub(super) fn new<S, T>(items: Signal<S>, row_key: Option<RowKey<T>>, estimate: f32) -> Self
    where
        S: Deref + Clone + 'static,
        S::Target: AsRef<[T]>,
        T: 'static,
    {
        let cache: Cache = Rc::default();
        let version = signal(0_u64);
        prune(items, row_key.clone(), Rc::clone(&cache));
        let keys = row_key.clone().map(|key_of| {
            derived(move || {
                let items = items.get();
                Rc::new(
                    bound_slice(&items)
                        .iter()
                        .enumerate()
                        .map(|(index, item)| key_of(index, item))
                        .collect::<Vec<u64>>(),
                )
            })
        });
        let heights = Rc::clone(&cache);
        let tops = derived(move || {
            version.track();
            let items = items.get();
            let slice = bound_slice(&items);
            let known = heights.borrow();
            let mut tops = Vec::with_capacity(slice.len().saturating_add(1));
            let mut cursor = 0.0;
            tops.push(cursor);
            for (index, item) in slice.iter().enumerate() {
                let key = row_key_of(row_key.as_ref(), index, item);
                cursor += known.get(&key).copied().unwrap_or(estimate);
                tops.push(cursor);
            }
            Rc::new(tops)
        });
        Self {
            cache,
            width: Rc::default(),
            version,
            tops,
            keys,
        }
    }

    fn record(&self, key: u64, (width, height): (f32, f32)) {
        if !width.is_finite() || width <= 0.0 || !height.is_finite() || height < 0.0 {
            return;
        }
        let mut cache = self.cache.borrow_mut();
        if (self.width.replace(width) - width).abs() > f32::EPSILON {
            cache.clear();
        }
        let previous = cache.insert(key, height);
        drop(cache);
        if previous.is_none_or(|old| (old - height).abs() > f32::EPSILON) {
            self.version.update(|version| *version += 1);
        }
    }

    pub(super) fn slot(
        &self,
        key: u64,
        width: WidthSpec,
        row: impl IntoNode + 'static,
    ) -> NodeBuilder {
        let size = signal((0.0_f32, 0.0_f32));
        let measure = self.clone();
        effect(move || measure.record(key, size.get()));
        container()
            .layout(width.apply(LayoutStyle::block()))
            .size_reactive(size)
            .child(row)
    }
}
