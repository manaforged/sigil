use super::scales::Fit;
use super::ticks::nice;
use super::{ChartMark, ChartView, Stack, nice_max};

const FLAT_PAD: f32 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ValueSpan {
    pub(super) lo: f32,
    pub(super) hi: f32,
}

impl ValueSpan {
    pub(super) fn of(series: &[ChartView<'_>], mark: ChartMark, stack: Stack, fit: Fit) -> Self {
        if let Some(span) = Self::given(fit) {
            return span;
        }
        let zero = fit.zero_or(mark != ChartMark::Scatter);
        match (mark, stack) {
            (ChartMark::Candle, _) => Self::tight(series.iter().flat_map(|item| item.values)),
            (ChartMark::Scatter, _) => Self::fitted(
                series
                    .iter()
                    .flat_map(|item| item.values.iter().skip(1).step_by(2)),
                zero,
            ),
            (ChartMark::Bar | ChartMark::Area, Stack::Stacked) => {
                Self::fitted(Self::totals(series).iter(), zero)
            }
            _ => Self::fitted(series.iter().flat_map(|item| item.values), zero),
        }
    }

    pub(super) fn scatter_x(series: &[ChartView<'_>], fit: Fit) -> Self {
        Self::given(fit).unwrap_or_else(|| {
            Self::fitted(
                series.iter().flat_map(|item| item.values.iter().step_by(2)),
                fit.zero_or(false),
            )
        })
    }

    pub(super) fn placed<'a>(at: impl Iterator<Item = &'a f32>, fit: Fit, time: bool) -> Self {
        Self::given(fit).unwrap_or_else(|| {
            if time {
                Self::tight(at)
            } else {
                Self::fitted(at, fit.zero_or(false))
            }
        })
    }

    fn given(fit: Fit) -> Option<Self> {
        fit.domain
            .filter(|(lo, hi)| lo.is_finite() && hi.is_finite() && hi > lo)
            .map(|(lo, hi)| Self { lo, hi })
    }

    fn totals(series: &[ChartView<'_>]) -> Vec<f32> {
        let count = series
            .iter()
            .map(|item| item.values.len())
            .max()
            .unwrap_or(0);
        (0..count)
            .flat_map(|index| {
                let (mut up, mut down) = (0.0_f32, 0.0_f32);
                for value in series.iter().filter_map(|item| item.values.get(index)) {
                    if value.is_finite() && *value > 0.0 {
                        up += value;
                    } else if value.is_finite() {
                        down += value;
                    }
                }
                [up, down]
            })
            .collect()
    }

    fn fitted<'a>(values: impl Iterator<Item = &'a f32>, zero: bool) -> Self {
        if !zero {
            let tight = Self::tight(values);
            return nice(tight.lo, tight.hi);
        }
        let (lo, hi) = Self::extent(values).unwrap_or((0.0, 0.0));
        let (lo, hi) = (lo.min(0.0), hi.max(0.0));
        nice(lo, if hi > lo { hi } else { 1.0 })
    }

    fn extent<'a>(values: impl Iterator<Item = &'a f32>) -> Option<(f32, f32)> {
        let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
        for &value in values.filter(|value| value.is_finite()) {
            lo = lo.min(value);
            hi = hi.max(value);
        }
        (lo.is_finite() && hi.is_finite()).then_some((lo, hi))
    }

    fn tight<'a>(values: impl Iterator<Item = &'a f32>) -> Self {
        let Some((lo, hi)) = Self::extent(values) else {
            return Self { lo: 0.0, hi: 1.0 };
        };
        if hi > lo {
            Self { lo, hi }
        } else {
            let pad = nice_max(lo.abs() * FLAT_PAD).max(1.0);
            Self {
                lo: lo - pad,
                hi: hi + pad,
            }
        }
    }

    pub(super) fn width(self) -> f32 {
        self.hi - self.lo
    }

    pub(super) fn fraction(self, value: f32) -> f32 {
        self.share(value).clamp(0.0, 1.0)
    }

    pub(super) fn share(self, value: f32) -> f32 {
        (value - self.lo) / self.width()
    }
}
