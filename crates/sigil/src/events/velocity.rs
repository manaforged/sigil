use core::cell::RefCell;
use core::time::Duration;

use web_time::Instant;

use crate::events::{PointerEvent, PointerPhase, PointerType};
use crate::geometry::Point;

const SAMPLES: usize = 8;
const POINTERS: usize = 10;
const HORIZON: Duration = Duration::from_millis(100);
const PAUSE: Duration = Duration::from_millis(40);

type Key = (PointerType, u64);

#[derive(Clone, Copy, Debug)]
struct Sample {
    at: Instant,
    position: Point,
}

#[derive(Clone, Copy, Debug)]
struct Ring {
    key: Key,
    samples: [Option<Sample>; SAMPLES],
    head: usize,
}

impl Ring {
    const fn new(key: Key) -> Self {
        Self {
            key,
            samples: [None; SAMPLES],
            head: 0,
        }
    }

    fn push(&mut self, sample: Sample) {
        if let Some(slot) = self.samples.get_mut(self.head) {
            *slot = Some(sample);
        }
        self.head = (self.head + 1) % SAMPLES;
    }

    fn newest_first(&self) -> impl Iterator<Item = Sample> + '_ {
        (1..=SAMPLES)
            .map(|back| self.samples.get((self.head + SAMPLES - back) % SAMPLES))
            .map_while(|sample| sample.copied().flatten())
    }

    fn velocity(&self) -> Point {
        let mut samples = self.newest_first();
        let Some(newest) = samples.next() else {
            return Point::ZERO;
        };
        let mut oldest = newest;
        for sample in samples {
            let gap = oldest.at.saturating_duration_since(sample.at);
            if gap > PAUSE || newest.at.saturating_duration_since(sample.at) > HORIZON {
                break;
            }
            oldest = sample;
        }
        let span = newest.at.saturating_duration_since(oldest.at).as_secs_f32();
        if span <= 0.0 {
            return Point::ZERO;
        }
        Point::new(
            (newest.position.x - oldest.position.x) / span,
            (newest.position.y - oldest.position.y) / span,
        )
    }

    fn newest_at(&self) -> Option<Instant> {
        self.newest_first().next().map(|sample| sample.at)
    }
}

#[derive(Debug)]
struct Tracker {
    rings: [Option<Ring>; POINTERS],
}

impl Tracker {
    const fn new() -> Self {
        Self {
            rings: [None; POINTERS],
        }
    }

    fn find(&self, key: Key) -> Option<usize> {
        self.rings
            .iter()
            .position(|ring| ring.as_ref().is_some_and(|ring| ring.key == key))
    }

    fn vacant(&self) -> Option<usize> {
        self.rings.iter().position(Option::is_none).or_else(|| {
            self.rings
                .iter()
                .enumerate()
                .filter_map(|(index, ring)| Some((index, ring.as_ref()?.newest_at()?)))
                .min_by_key(|&(_, at)| at)
                .map(|(index, _)| index)
        })
    }

    fn open(&mut self, key: Key) -> Option<&mut Ring> {
        let index = self.find(key).or_else(|| self.vacant())?;
        let slot = self.rings.get_mut(index)?;
        if slot.as_ref().is_none_or(|ring| ring.key != key) {
            *slot = Some(Ring::new(key));
        }
        slot.as_mut()
    }

    fn close(&mut self, key: Key, last: Option<Sample>) -> Option<Point> {
        let index = self.find(key)?;
        let mut ring = self.rings.get_mut(index)?.take()?;
        let sample = last?;
        ring.push(sample);
        Some(ring.velocity())
    }

    fn record(&mut self, key: Key, phase: PointerPhase, sample: Sample) -> Option<Point> {
        match phase {
            PointerPhase::Down => {
                let ring = self.open(key)?;
                *ring = Ring::new(key);
                ring.push(sample);
                None
            }
            PointerPhase::Move => {
                let ring = self.open(key)?;
                ring.push(sample);
                Some(ring.velocity())
            }
            PointerPhase::Up => self.close(key, Some(sample)),
            PointerPhase::Cancel => self.close(key, None),
            PointerPhase::Enter | PointerPhase::Leave => None,
        }
    }
}

thread_local! {
    static TRACKER: RefCell<Tracker> = const { RefCell::new(Tracker::new()) };
}

pub(crate) fn track(event: &mut PointerEvent) {
    let at = crate::reactive::frame::now();
    let (key, phase) = ((event.kind, event.pointer_id), event.phase);
    let sample = Sample {
        at,
        position: event.window_position,
    };
    event.velocity = TRACKER
        .try_with(|tracker| tracker.borrow_mut().record(key, phase, sample))
        .ok()
        .flatten()
        .unwrap_or(Point::ZERO);
}
