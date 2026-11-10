use smallvec::SmallVec;

use super::{Keyframes, Property};
use crate::animation::Easing;
use crate::animation::runtime::binding_animate_color::mix;
use crate::style::{Color, ColorAtom, ColorAtomKind, NumericAtomKind};

type Stops<V> = SmallVec<[(f32, V); 8]>;

impl Keyframes {
    pub(crate) fn sample(
        &self,
        progress: f32,
        kind: NumericAtomKind,
        underlying: f32,
    ) -> Option<f32> {
        let stops = self.stops(|property| property.numeric(kind), underlying)?;
        let (from, to, eased) = segment(&stops, progress, |offset| self.easing_at(offset))?;
        Some((to - from).mul_add(eased, from))
    }

    pub(crate) fn sample_color(
        &self,
        progress: f32,
        kind: ColorAtomKind,
        underlying: Color,
        resolve: &dyn Fn(ColorAtom) -> Color,
    ) -> Option<Color> {
        let stops = self.stops(|property| property.color(kind).map(resolve), underlying)?;
        let (from, to, eased) = segment(&stops, progress, |offset| self.easing_at(offset))?;
        Some(mix(from, to, eased))
    }

    fn stops<V: Copy>(
        &self,
        value: impl Fn(Property) -> Option<V>,
        underlying: V,
    ) -> Option<Stops<V>> {
        let mut found: Stops<V> = self
            .frames
            .iter()
            .filter_map(|(offset, property)| value(*property).map(|v| (*offset, v)))
            .collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut stops: Stops<V> = SmallVec::new();
        for (offset, v) in found {
            match stops.last_mut() {
                Some(last) if last.0.to_bits() == offset.to_bits() => last.1 = v,
                _ => stops.push((offset, v)),
            }
        }
        let first = stops.first()?.0;
        if first > 0.0 {
            stops.insert(0, (0.0, underlying));
        }
        if stops.last().is_some_and(|last| last.0 < 1.0) {
            stops.push((1.0, underlying));
        }
        Some(stops)
    }
}

fn segment<V: Copy>(
    stops: &[(f32, V)],
    progress: f32,
    easing: impl Fn(f32) -> Easing,
) -> Option<(V, V, f32)> {
    let last = stops.len().saturating_sub(2);
    let index = stops
        .partition_point(|(offset, _)| *offset <= progress)
        .saturating_sub(1)
        .min(last);
    let &(start, from) = stops.get(index)?;
    let (end, to) = stops.get(index + 1).copied().unwrap_or((start, from));
    let span = end - start;
    let local = if span > 0.0 {
        ((progress - start) / span).clamp(0.0, 1.0)
    } else {
        1.0
    };
    Some((from, to, easing(start).apply(local)))
}
