use crate::app::FrameHooks;
use crate::events::ShellEvent;
use crate::geometry::{Point, Rect};
use crate::profile;
use crate::scene::CommandScene;
use crate::scene::damage::{Changed, Damage as SceneDamage, FullReason, cell_damage};
use crate::terminal::paint::{Marks, Paint, raster_regions, raster_with};
use crate::terminal::run::view;
use crate::terminal::write::Damage;

use super::{CELL, Screen};

pub(super) fn frame_hooks<'s>(
    screen: &'s mut Screen,
    events: impl IntoIterator<Item = ShellEvent> + 's,
    size: (u16, u16),
) -> FrameHooks<'s> {
    let (cols, rows) = size;
    FrameHooks {
        input: Some(Box::new(move |host, router| {
            let _phase = profile::span(profile::Phase::Effect);
            for event in events {
                profile::interact(profile::Cause::Input);
                profile::count(profile::Counter::TuiEvents);
                host.route(router, event);
            }
            None
        })),
        paint: Some(Box::new(move |host, router, info| {
            paint(screen, (host, router), info, (cols, rows));
        })),
        ..FrameHooks::default()
    }
}

fn paint(
    screen: &mut Screen,
    (host, router): (&mut crate::Host, &mut crate::events::EventRouter),
    info: &crate::app::FrameInfo,
    (cols, rows): (u16, u16),
) {
    let _phase = profile::span(profile::Phase::TuiPaint);
    if !info.layout_ok {
        host.reset_emit();
        return;
    }
    let resized =
        screen.grid.cols() != usize::from(cols) || screen.grid.rows() != usize::from(rows);
    if !resized
        && !screen.fresh
        && host.emit_unchanged(view(cols, rows), &info.delta, &screen.scene)
    {
        screen.dirty = false;
        return;
    }
    let (next, diff) = host.emit_since(view(cols, rows), &info.delta, &screen.scene);
    screen.dirty = !diff.unchanged() || resized || screen.fresh;
    if !screen.dirty && info.delta.is_empty() {
        return;
    }
    let reuse = diff.unchanged() && !screen.marks.stale(host.cx(), router.focus(), &info.delta);
    let (marks, relinked) = if reuse {
        (core::mem::take(&mut screen.marks), false)
    } else {
        let marks = Marks::collect(host.cx(), router.focus(), &next, CELL);
        let relinked = !marks.same_links(&screen.marks);
        (marks, relinked)
    };
    screen.dirty |= relinked;
    if screen.dirty {
        let _raster = profile::span(profile::Phase::TuiRaster);
        let paint = Paint {
            theme: host.theme(),
            cell: CELL,
            caps: screen.caps,
            marks: &marks,
        };
        raster(
            screen,
            (&next, &diff.changed),
            &paint,
            (cols, rows),
            relinked,
        );
        screen.scene = next;
    }
    screen.marks = marks;
}

fn raster(
    screen: &mut Screen,
    (next, changed): (&CommandScene, &[Changed]),
    paint: &Paint<'_>,
    (cols, rows): (u16, u16),
    whole: bool,
) {
    let retained = !whole
        && !screen.fresh
        && screen.prev.cols() == usize::from(cols)
        && screen.prev.rows() == usize::from(rows)
        && screen.prev.images().is_empty();
    let area = Rect::new(
        Point::new(0.0, 0.0),
        f32::from(cols) * CELL,
        f32::from(rows) * CELL,
    );
    let damage = if retained {
        cell_damage((&screen.scene, next), Some(changed), area, (CELL, CELL))
    } else {
        SceneDamage::Full(FullReason::NoBackbuffer)
    };
    match damage {
        SceneDamage::Regions(regions) => {
            screen.grid.sync_rows(&screen.prev, screen.stale.as_deref());
            let restored = screen.announcer.restore(&mut screen.grid);
            let mut touched = raster_regions(&mut screen.grid, next, paint, &regions);
            touched.extend(restored);
            screen.damage = Some(Damage::new(touched, false));
        }
        SceneDamage::Unchanged | SceneDamage::Full(_) => {
            screen.grid.resize(cols, rows);
            raster_with(&mut screen.grid, next, paint);
        }
    }
}
