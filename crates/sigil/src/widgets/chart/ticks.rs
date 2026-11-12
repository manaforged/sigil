use super::span::ValueSpan;

const NICE_COUNT: f64 = 10.0;
const PX_PER_TICK: f32 = 40.0;
pub(super) const MIN_TICKS: usize = 2;
const E10: f64 = 7.071_067_811_865_476;
const E5: f64 = 3.162_277_660_168_379_5;
const E2: f64 = std::f64::consts::SQRT_2;
const NICE_ROUNDS: usize = 10;
const GROUP_FROM: f64 = 1_000.0;
const EPSILON: f64 = 1e-9;
const F32_SLACK: f64 = 1e-6;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Tick {
    pub(super) at: f32,
    pub(super) text: String,
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a tick count from a pixel length is small and positive"
)]
pub(super) fn count(px: f32) -> usize {
    ((px / PX_PER_TICK).ceil().max(0.0) as usize).max(MIN_TICKS)
}

pub(super) fn step(lo: f64, hi: f64, count: f64) -> f64 {
    let raw = (hi - lo).abs() / count.max(1.0);
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let power = 10f64.powf(raw.log10().floor());
    let error = raw / power;
    let factor = if error >= E10 {
        10.0
    } else if error >= E5 {
        5.0
    } else if error >= E2 {
        2.0
    } else {
        1.0
    };
    factor * power
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "chart spans render at f32 precision"
)]
#[allow(
    clippy::float_cmp,
    reason = "the loop stops when the step reaches an exact fixed point"
)]
pub(super) fn nice(lo: f32, hi: f32) -> ValueSpan {
    let (mut lo, mut hi) = (f64::from(lo), f64::from(hi));
    let mut last = f64::NAN;
    for _ in 0..NICE_ROUNDS {
        let unit = step(lo, hi, NICE_COUNT);
        if unit == last {
            break;
        }
        lo = (lo / unit + F32_SLACK).floor() * unit;
        hi = (hi / unit - F32_SLACK).ceil() * unit;
        last = unit;
    }
    ValueSpan {
        lo: lo as f32,
        hi: hi as f32,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "tick indexes and fractions are small"
)]
pub(super) fn ticks(span: ValueSpan, count: usize, whole: bool) -> Vec<Tick> {
    let (lo, hi) = (f64::from(span.lo), f64::from(span.hi));
    let mut unit = step(lo, hi, count_f64(count));
    if whole {
        unit = unit.max(1.0);
    }
    let first = (lo / unit - EPSILON).ceil() as i64;
    let last = (hi / unit + EPSILON).floor() as i64;
    let decimals = decimals(unit);
    (first..=last)
        .map(|index| {
            let value = index_f64(index) * unit;
            Tick {
                at: ((value - lo) / (hi - lo)) as f32,
                text: label(value, decimals),
            }
        })
        .collect()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "tick counts and indexes are small"
)]
const fn count_f64(count: usize) -> f64 {
    count as f64
}

#[allow(
    clippy::cast_precision_loss,
    reason = "tick counts and indexes are small"
)]
const fn index_f64(index: i64) -> f64 {
    index as f64
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the decimal count is a small non-negative number"
)]
fn decimals(unit: f64) -> usize {
    (-(unit.log10() + EPSILON).floor()).max(0.0) as usize
}

fn label(value: f64, decimals: usize) -> String {
    let fixed = format!("{value:.decimals$}");
    let fixed = fixed.strip_prefix('-').map_or_else(
        || fixed.clone(),
        |rest| {
            if rest.chars().all(|c| c == '0' || c == '.') {
                rest.to_owned()
            } else {
                fixed.clone()
            }
        },
    );
    if value.abs() < GROUP_FROM {
        return fixed;
    }
    group(&fixed)
}

fn group(fixed: &str) -> String {
    let (sign, digits) = fixed
        .strip_prefix('-')
        .map_or(("", fixed), |rest| ("-", rest));
    let (whole, fraction) = digits
        .split_once('.')
        .map_or((digits, None), |(whole, fraction)| (whole, Some(fraction)));
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    fraction.map_or_else(
        || format!("{sign}{grouped}"),
        |fraction| format!("{sign}{grouped}.{fraction}"),
    )
}
