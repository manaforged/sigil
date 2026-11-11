use std::ops::RangeInclusive;

use crate::reactive::{Signal, derived};

#[derive(Clone, Copy, Debug)]
pub(super) enum Thumb {
    Only(Signal<f32>),
    Start(Signal<RangeInclusive<f32>>),
    End(Signal<RangeInclusive<f32>>),
}

impl Thumb {
    pub(super) fn get(self) -> f32 {
        match self {
            Self::Only(value) => value.get(),
            Self::Start(range) => *range.get().start(),
            Self::End(range) => *range.get().end(),
        }
    }

    pub(super) fn peek(self) -> f32 {
        match self {
            Self::Only(value) => value.peek(),
            Self::Start(range) => *range.peek().start(),
            Self::End(range) => *range.peek().end(),
        }
    }

    pub(super) fn set(self, next: f32) {
        match self {
            Self::Only(value) => value.set(next),
            Self::Start(range) => {
                let end = *range.peek().end();
                range.set(next.min(end)..=end);
            }
            Self::End(range) => {
                let start = *range.peek().start();
                range.set(start..=next.max(start));
            }
        }
    }

    pub(super) fn signal(self) -> Signal<f32> {
        match self {
            Self::Only(value) => value,
            _ => derived(move || self.get()),
        }
    }

    pub(super) fn nearest(self, at: f32) -> Self {
        match self {
            Self::Only(_) => self,
            Self::Start(range) | Self::End(range) => {
                let (start, end) = range.peek().into_inner();
                let toward_start = at < start || (at <= end && at - start < end - at);
                if toward_start {
                    Self::Start(range)
                } else {
                    Self::End(range)
                }
            }
        }
    }

    pub(super) fn overlapped(self) -> bool {
        match self {
            Self::Start(range) | Self::End(range) => {
                let (start, end) = range.peek().into_inner();
                end <= start
            }
            Self::Only(_) => false,
        }
    }

    pub(super) fn toward(self, at: f32) -> Self {
        match self {
            Self::Start(range) | Self::End(range) if self.overlapped() => {
                let (start, end) = range.peek().into_inner();
                if at < start {
                    Self::Start(range)
                } else if at > end {
                    Self::End(range)
                } else {
                    self
                }
            }
            _ => self,
        }
    }

    pub(super) fn holds(self, at: f32) -> bool {
        let near = |value: f32| (value - at).abs() < f32::EPSILON;
        match self {
            Self::Only(value) => near(value.get()),
            Self::Start(range) | Self::End(range) => {
                let (start, end) = range.get().into_inner();
                near(start) || near(end)
            }
        }
    }

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Only(_) | Self::Start(_) => 0,
            Self::End(_) => 1,
        }
    }
}
