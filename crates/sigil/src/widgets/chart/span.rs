use super::{ChartMark, ChartView, Stack, nice_max};

const NICE_TICKS: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ValueSpan {
    pub(super) lo: f32,
    pub(super) hi: f32,
}

impl ValueSpan {
    pub(super) fn of(series: &[ChartView<'_>], mark: ChartMark, stack: Stack) -> Self {
        match (mark, stack) {
            (ChartMark::Candle, _) => Self::tight(series.iter().flat_map(|item| item.values)),
            (ChartMark::Scatter, _) => Self::rounded(Self::tight(
                series
                    .iter()
                    .flat_map(|item| item.values.iter().skip(1).step_by(2)),
            )),
            (ChartMark::Bar, Stack::Stacked) => Self::stacked(series),
            _ => Self::zero_based(series.iter().flat_map(|item| item.values)),
        }
    }

    pub(super) fn scatter_x(series: &[ChartView<'_>]) -> Self {
        Self::rounded(Self::tight(
            series.iter().flat_map(|item| item.values.iter().step_by(2)),
        ))
    }

    fn zero_based<'a>(values: impl Iterator<Item = &'a f32>) -> Self {
        let (mut peak, mut trough) = (0.0_f32, 0.0_f32);
        for &value in values.filter(|value| value.is_finite()) {
            peak = peak.max(value);
            trough = trough.min(value);
        }
        Self {
            lo: if trough < 0.0 {
                -nice_max(-trough)
            } else {
                0.0
            },
            hi: if peak > 0.0 || trough >= 0.0 {
                nice_max(peak)
            } else {
                0.0
            },
        }
    }

    fn stacked(series: &[ChartView<'_>]) -> Self {
        let count = series
            .iter()
            .map(|item| item.values.len())
            .max()
            .unwrap_or(0);
        let totals = (0..count).flat_map(|index| {
            let (mut up, mut down) = (0.0_f32, 0.0_f32);
            for value in series.iter().filter_map(|item| item.values.get(index)) {
                if value.is_finite() && *value > 0.0 {
                    up += value;
                } else if value.is_finite() {
                    down += value;
                }
            }
            [up, down]
        });
        Self::zero_based(totals.collect::<Vec<_>>().iter())
    }

    fn tight<'a>(values: impl Iterator<Item = &'a f32>) -> Self {
        let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
        for &value in values.filter(|value| value.is_finite()) {
            lo = lo.min(value);
            hi = hi.max(value);
        }
        if !(lo.is_finite() && hi.is_finite()) {
            return Self { lo: 0.0, hi: 1.0 };
        }
        if hi > lo {
            Self { lo, hi }
        } else {
            Self {
                lo: lo - 1.0,
                hi: hi + 1.0,
            }
        }
    }

    fn rounded(self) -> Self {
        let step = nice_max(self.width() / NICE_TICKS);
        Self {
            lo: (self.lo / step).floor() * step,
            hi: (self.hi / step).ceil() * step,
        }
    }

    pub(super) fn width(self) -> f32 {
        self.hi - self.lo
    }

    pub(super) fn fraction(self, value: f32) -> f32 {
        ((value - self.lo) / self.width()).clamp(0.0, 1.0)
    }
}
