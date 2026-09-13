use std::cell::RefCell;

use crate::reactive::Signal;
use crate::style::Rgba8;

use super::spawn::{FctSpawn, Particle};
use super::style::FloaterStyle;

pub const FCT_POOL_CAPACITY: usize = 1024;

#[derive(Clone, Copy, Debug)]
pub(super) struct FctSettings {
    pub(super) duration: f32,
    pub(super) drift: [f32; 2],
    pub(super) time: Option<Signal<f32>>,
    pub(super) capacity: usize,
    pub(super) anchor: Option<Signal<(f32, f32)>>,
    pub(super) style: Option<FloaterStyle>,
}

impl FctSettings {
    pub(super) const fn new() -> Self {
        Self {
            duration: 1.2,
            drift: [0.0, -32.0],
            time: None,
            capacity: FCT_POOL_CAPACITY,
            anchor: None,
            style: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct FctBuildOptions {
    anchor: Option<Signal<(f32, f32)>>,
    override_color: Option<Rgba8>,
}

#[derive(Debug)]
pub struct FctState {
    pool: RefCell<Vec<Particle>>,
    duration: f32,
    drift: [f32; 2],
    last_tick: RefCell<f32>,
    anchor: Option<Signal<(f32, f32)>>,
    override_color: Option<Rgba8>,
    pub generation: Signal<u32>,
}

impl FctState {
    #[must_use]
    pub fn new(duration: f32, drift: [f32; 2], time: Signal<f32>, capacity: usize) -> Self {
        Self::build(duration, drift, time, capacity, FctBuildOptions::default())
    }

    #[must_use]
    pub fn new_anchored(
        duration: f32,
        drift: [f32; 2],
        time: Signal<f32>,
        capacity: usize,
        anchor: Signal<(f32, f32)>,
    ) -> Self {
        Self::build(
            duration,
            drift,
            time,
            capacity,
            FctBuildOptions {
                anchor: Some(anchor),
                override_color: None,
            },
        )
    }

    fn build(
        duration: f32,
        drift: [f32; 2],
        time: Signal<f32>,
        capacity: usize,
        opts: FctBuildOptions,
    ) -> Self {
        let pool = (0..capacity).map(|_| Particle::empty()).collect();
        Self {
            pool: RefCell::new(pool),
            duration: duration.max(f32::EPSILON),
            drift,
            last_tick: RefCell::new(time.peek()),
            anchor: opts.anchor,
            override_color: opts.override_color,
            generation: crate::reactive::signal(0_u32),
        }
    }

    pub(super) fn from_settings(settings: FctSettings, time: Signal<f32>) -> Self {
        let opts = FctBuildOptions {
            anchor: settings.anchor,
            override_color: settings.style.map(FloaterStyle::color),
        };
        Self::build(
            settings.duration,
            settings.drift,
            time,
            settings.capacity,
            opts,
        )
    }

    pub fn spawn(&self, spawn: FctSpawn, now: f32) -> bool {
        if spawn.text.is_empty() {
            return false;
        }
        let mut pool = self.pool.borrow_mut();
        let Some(slot) = pool.iter_mut().find(|p| !p.alive) else {
            return false;
        };
        let color = self.override_color.unwrap_or(spawn.color);
        *slot = Particle {
            origin: spawn.at,
            drift: self.drift,
            position: spawn.at,
            spawn_time: now,
            remaining: self.duration,
            alpha: 1.0,
            color,
            text: spawn.text,
            alive: true,
        };
        true
    }

    pub fn tick(&self, now: f32) {
        let dt = {
            let mut last = self.last_tick.borrow_mut();
            let dt = (now - *last).max(0.0);
            *last = now;
            dt
        };
        if dt == 0.0 {
            return;
        }
        let inv_dur = 1.0 / self.duration;
        let anchor_now = self.anchor.map(Signal::get);
        let mut pool = self.pool.borrow_mut();
        for slot in pool.iter_mut() {
            advance_slot(slot, dt, inv_dur, now, anchor_now);
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<Particle> {
        self.pool
            .borrow()
            .iter()
            .filter(|p| p.alive)
            .cloned()
            .collect()
    }

    pub fn for_each_alive(&self, mut visit: impl FnMut(&Particle)) {
        for slot in self.pool.borrow().iter() {
            if slot.alive {
                visit(slot);
            }
        }
    }

    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.pool.borrow().iter().filter(|p| p.alive).count()
    }
}

fn advance_slot(slot: &mut Particle, dt: f32, inv_dur: f32, now: f32, anchor: Option<(f32, f32)>) {
    if !slot.alive {
        return;
    }
    slot.remaining -= dt;
    if slot.remaining <= 0.0 {
        *slot = Particle::empty();
        return;
    }
    if let Some((anchor_x, anchor_y)) = anchor {
        let elapsed = (now - slot.spawn_time).max(0.0);
        slot.position[0] = slot.drift[0].mul_add(elapsed, anchor_x);
        slot.position[1] = slot.drift[1].mul_add(elapsed, anchor_y);
    } else {
        slot.position[0] = slot.drift[0].mul_add(dt, slot.position[0]);
        slot.position[1] = slot.drift[1].mul_add(dt, slot.position[1]);
    }
    slot.alpha = (slot.remaining * inv_dur).clamp(0.0, 1.0);
}
