use core::time::Duration;

use sigil_reactive::{IntervalId, register_interval};

use crate::Cx;

impl Cx {
    #[track_caller]
    pub fn interval(&self, period: Duration, callback: impl Fn() + 'static) -> IntervalId {
        let _ = self;
        register_interval(period, callback)
    }
}

#[cfg(test)]
#[path = "interval_tests.rs"]
mod tests;
