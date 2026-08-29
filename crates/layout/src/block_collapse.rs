#![allow(clippy::redundant_pub_crate)]

pub(crate) fn collapse(a: f32, b: f32) -> f32 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else if a < 0.0 && b < 0.0 {
        a.min(b)
    } else {
        a + b
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OpenMargin {
    pub(crate) value: f32,
    pub(crate) active: bool,
}

impl OpenMargin {
    pub(crate) const fn sealed() -> Self {
        Self {
            value: 0.0,
            active: false,
        }
    }

    pub(crate) fn absorb_top(&mut self, top: f32) -> f32 {
        if self.active {
            let merged = collapse(self.value, top);
            self.value = 0.0;
            self.active = false;
            merged
        } else {
            self.value = 0.0;
            top
        }
    }

    pub(crate) const fn emit_bottom(&mut self, bottom: f32) {
        self.value = bottom;
        self.active = true;
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "block_collapse_tests.rs"]
mod tests;
