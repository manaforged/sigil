use core::ops::Range;

use smallvec::SmallVec;

mod changed;
mod reason;
mod regions;

#[cfg(feature = "scene")]
pub(crate) use changed::command_ink;
pub(crate) use reason::FullReason;
pub(crate) use regions::Budget;

use crate::geometry::Rect;
use crate::scene::{CommandScene, LayerEffect, SceneCommand};

const MAX_CHANGED_SHARE: usize = 4;
const MIN_SHARE_SCENE: usize = 64;
const RESYNC_WINDOW: usize = 16;

#[derive(Debug, PartialEq)]
pub(crate) enum Damage<R> {
    Unchanged,
    Regions(R),
    Full(FullReason),
}

#[cfg_attr(not(test), allow(dead_code, reason = "the terminal screen wires it"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChangedRange {
    pub(crate) old: Range<usize>,
    pub(crate) new: Range<usize>,
}

#[cfg_attr(not(test), allow(dead_code, reason = "the terminal screen wires it"))]
#[derive(Clone, Copy)]
pub(crate) enum Changes<'a> {
    Diff,
    Ranges(&'a [ChangedRange]),
}

type Inks = SmallVec<[Rect; 16]>;

#[cfg_attr(
    not(feature = "scene"),
    expect(dead_code, reason = "the GPU renderer plans regions with it")
)]
pub(crate) fn scene_damage<R: FromIterator<Rect>>(
    (previous, next): (&CommandScene, &CommandScene),
    changes: Changes<'_>,
    (area, scale): (Rect, f32),
    budget: Budget,
) -> Damage<R> {
    match changed_inks(previous, next, changes, (area, scale)) {
        Damage::Regions(inks) => regions::regions(&inks, area, scale, budget),
        Damage::Unchanged => Damage::Unchanged,
        Damage::Full(reason) => Damage::Full(reason),
    }
}

const TERMINAL_SCALE: f32 = 1.0;

#[cfg_attr(not(test), allow(dead_code, reason = "the terminal screen wires it"))]
pub(crate) fn cell_damage(
    (previous, next): (&CommandScene, &CommandScene),
    changes: Changes<'_>,
    area: Rect,
    cell: (f32, f32),
) -> Damage<Vec<Rect>> {
    match changed_inks(previous, next, changes, (area, TERMINAL_SCALE)) {
        Damage::Regions(inks) => regions::cells(&inks, area, cell, TERMINAL_SCALE.recip()),
        Damage::Unchanged => Damage::Unchanged,
        Damage::Full(reason) => Damage::Full(reason),
    }
}

fn changed_inks(
    previous: &CommandScene,
    next: &CommandScene,
    changes: Changes<'_>,
    (area, scale): (Rect, f32),
) -> Damage<Inks> {
    let (before, after) = (&previous.commands, &next.commands);
    if after.iter().any(
        |command| matches!(command, SceneCommand::PushLayer(scope) if !local_effect(scope.effect)),
    ) {
        return Damage::Full(FullReason::NonLocalLayerEffect);
    }
    let Some((old, new)) = marks(before, after, changes) else {
        return Damage::Unchanged;
    };
    let marked = old.iter().chain(&new).filter(|mark| **mark).count();
    if after.len() >= MIN_SHARE_SCENE && marked * MAX_CHANGED_SHARE > after.len() * 2 {
        return Damage::Full(FullReason::ChangedShare);
    }
    let mut inks = Inks::new();
    for (commands, marks) in [(before, &old), (after, &new)] {
        let end = marks
            .iter()
            .rposition(|mark| *mark)
            .map_or(0, |last| last + 1);
        let flagged = |index: usize| marks.get(index).copied().unwrap_or(false);
        if changed::changed_ink(commands, end, &flagged, (area, scale), &mut inks).is_none() {
            return Damage::Full(FullReason::UnboundedChange);
        }
    }
    Damage::Regions(inks)
}

fn marks(
    before: &[SceneCommand],
    after: &[SceneCommand],
    changes: Changes<'_>,
) -> Option<(Vec<bool>, Vec<bool>)> {
    match changes {
        Changes::Diff => diff_marks(before, after),
        Changes::Ranges([]) => None,
        Changes::Ranges(ranges) => {
            let mut old = vec![false; before.len()];
            let mut new = vec![false; after.len()];
            for range in ranges {
                mark(&mut old, clamp(&range.old, before.len()));
                mark(&mut new, clamp(&range.new, after.len()));
            }
            Some((old, new))
        }
    }
}

fn diff_marks(before: &[SceneCommand], after: &[SceneCommand]) -> Option<(Vec<bool>, Vec<bool>)> {
    let prefix = before.iter().zip(after).take_while(|(a, b)| a == b).count();
    if prefix == before.len() && prefix == after.len() {
        return None;
    }
    let room = before.len().min(after.len()) - prefix;
    let suffix = before
        .iter()
        .rev()
        .zip(after.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    Some(align(before, after, prefix, suffix))
}

fn align(
    before: &[SceneCommand],
    after: &[SceneCommand],
    prefix: usize,
    suffix: usize,
) -> (Vec<bool>, Vec<bool>) {
    let mut old = vec![false; before.len()];
    let mut new = vec![false; after.len()];
    let (end_old, end_new) = (before.len() - suffix, after.len() - suffix);
    let (mut i, mut j) = (prefix, prefix);
    while i < end_old && j < end_new {
        if before.get(i) == after.get(j) {
            i += 1;
            j += 1;
            continue;
        }
        let skip = (1..=RESYNC_WINDOW).find_map(|step| {
            if i + step < end_old && before.get(i + step) == after.get(j) {
                Some((step, 0))
            } else if j + step < end_new && before.get(i) == after.get(j + step) {
                Some((0, step))
            } else {
                None
            }
        });
        let (skip_old, skip_new) = skip.unwrap_or((1, 1));
        mark(&mut old, i..i + skip_old);
        mark(&mut new, j..j + skip_new);
        i += skip_old;
        j += skip_new;
    }
    mark(&mut old, i..end_old);
    mark(&mut new, j..end_new);
    (old, new)
}

fn mark(marks: &mut [bool], range: Range<usize>) {
    if let Some(marks) = marks.get_mut(range) {
        marks.fill(true);
    }
}

fn clamp(range: &Range<usize>, len: usize) -> Range<usize> {
    range.start.min(len)..range.end.min(len)
}

fn local_effect(effect: LayerEffect) -> bool {
    match effect {
        LayerEffect::Identity => true,
        LayerEffect::Transform([[_, skew_x, _], [skew_y, _, _]]) => skew_x == 0.0 && skew_y == 0.0,
        LayerEffect::LayerBlur { .. } => false,
    }
}

#[cfg(test)]
#[path = "damage_tests.rs"]
mod tests;
