use crate::widgets::Date;

use super::span::ValueSpan;
use super::ticks::{Tick, step};

pub(super) const DAY_MS: f64 = 86_400_000.0;
const SECOND: f64 = 1_000.0;
const MINUTE: f64 = 60.0 * SECOND;
const HOUR: f64 = 60.0 * MINUTE;
const WEEK: f64 = 7.0 * DAY_MS;
const MONTH: f64 = 30.0 * DAY_MS;
const YEAR: f64 = 365.0 * DAY_MS;
const SUNDAY_OFFSET: i64 = 4;
const MAX_TICKS: usize = 400;
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Every {
    Fixed(f64),
    Days(i64),
    Week,
    Months(u32),
    Years(i64),
}

const INTERVALS: [(Every, f64); 17] = [
    (Every::Fixed(SECOND), SECOND),
    (Every::Fixed(5.0 * SECOND), 5.0 * SECOND),
    (Every::Fixed(15.0 * SECOND), 15.0 * SECOND),
    (Every::Fixed(30.0 * SECOND), 30.0 * SECOND),
    (Every::Fixed(MINUTE), MINUTE),
    (Every::Fixed(5.0 * MINUTE), 5.0 * MINUTE),
    (Every::Fixed(15.0 * MINUTE), 15.0 * MINUTE),
    (Every::Fixed(30.0 * MINUTE), 30.0 * MINUTE),
    (Every::Fixed(HOUR), HOUR),
    (Every::Fixed(3.0 * HOUR), 3.0 * HOUR),
    (Every::Fixed(6.0 * HOUR), 6.0 * HOUR),
    (Every::Fixed(12.0 * HOUR), 12.0 * HOUR),
    (Every::Days(1), DAY_MS),
    (Every::Days(2), 2.0 * DAY_MS),
    (Every::Week, WEEK),
    (Every::Months(1), MONTH),
    (Every::Months(3), 3.0 * MONTH),
];

pub(super) fn days(date: Date) -> i64 {
    let year = i64::from(date.year) - i64::from(date.month <= 2);
    let era = year.div_euclid(400);
    let of_era = year.rem_euclid(400);
    let month = i64::from(date.month);
    let of_year =
        (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(date.day) - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a civil month and day are small and positive"
)]
fn civil(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let of_cycle = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * of_cycle + of_cycle / 4 - of_cycle / 100);
    let shape = (5 * of_year + 2) / 153;
    let day = (of_year - (153 * shape + 2) / 5 + 1) as u32;
    let month = if shape < 10 { shape + 3 } else { shape - 9 } as u32;
    let year = of_cycle + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a day count from a calendar year is far below f64 precision"
)]
fn day_start(days: i64) -> f64 {
    days as f64 * DAY_MS
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a calendar year fits in i32"
)]
fn month_start(year: i64, month: u32) -> f64 {
    let date = Date {
        year: year as i32,
        month: u8::try_from(month).unwrap_or(1),
        day: 1,
    };
    day_start(days(date))
}

fn pick(target: f64, lo: f64, hi: f64, count: f64) -> Every {
    let at = INTERVALS.iter().position(|(_, span)| *span > target);
    match at {
        None => Every::Years(year_step(lo, hi, count)),
        Some(0) => Every::Fixed(step(lo, hi, count).max(1.0)),
        Some(index) => {
            let (below, above) = (INTERVALS[index - 1], INTERVALS[index]);
            if target / below.1 < above.1 / target {
                below.0
            } else {
                above.0
            }
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a year step is a small positive count"
)]
fn year_step(lo: f64, hi: f64, count: f64) -> i64 {
    (step(lo / YEAR, hi / YEAR, count).round() as i64).max(1)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "tick instants stay within the parsed date range"
)]
fn instants(every: Every, lo: f64, hi: f64) -> Vec<f64> {
    let first_day = (lo / DAY_MS).floor() as i64;
    match every {
        Every::Fixed(unit) => {
            let mut at = (lo / unit).ceil() * unit;
            let mut out = Vec::new();
            while at <= hi && out.len() < MAX_TICKS {
                out.push(at);
                at += unit;
            }
            out
        }
        Every::Days(gap) => calendar_days(first_day, lo, hi, |day| (civil(day).2 - 1) as i64 % gap == 0),
        Every::Week => calendar_days(first_day, lo, hi, |day| (day + SUNDAY_OFFSET).rem_euclid(7) == 0),
        Every::Months(gap) => calendar_months(lo, hi, |_, month| (month - 1) % gap == 0),
        Every::Years(gap) => calendar_months(lo, hi, |year, month| month == 1 && year % gap == 0),
    }
}

fn calendar_days(first: i64, lo: f64, hi: f64, keep: impl Fn(i64) -> bool) -> Vec<f64> {
    (first..)
        .map(|day| (day, day_start(day)))
        .take_while(|(_, at)| *at <= hi)
        .filter(|(day, at)| *at >= lo && keep(*day))
        .map(|(_, at)| at)
        .take(MAX_TICKS)
        .collect()
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "tick instants stay within the parsed date range"
)]
fn calendar_months(lo: f64, hi: f64, keep: impl Fn(i64, u32) -> bool) -> Vec<f64> {
    let (mut year, mut month, _) = civil((lo / DAY_MS).floor() as i64);
    let mut out = Vec::new();
    loop {
        let at = month_start(year, month);
        if at > hi || out.len() >= MAX_TICKS {
            return out;
        }
        if at >= lo && keep(year, month) {
            out.push(at);
        }
        (year, month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    }
}

pub(super) fn ticks(origin: f64, span: ValueSpan, count: usize) -> Vec<Tick> {
    let (lo, hi) = (origin + f64::from(span.lo), origin + f64::from(span.hi));
    let count = f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    let every = pick((hi - lo) / count.max(1.0), lo, hi, count);
    instants(every, lo, hi)
        .into_iter()
        .map(|at| Tick {
            at: share(at, lo, hi),
            text: format(at),
        })
        .collect()
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a tick fraction renders at f32 precision"
)]
fn share(at: f64, lo: f64, hi: f64) -> f32 {
    ((at - lo) / (hi - lo)) as f32
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a time of day fits the small integer fields"
)]
pub(super) fn format(at: f64) -> String {
    let day = (at / DAY_MS).floor() as i64;
    let clock = at - day_start(day);
    let (ms, seconds) = ((clock % SECOND) as u32, (clock / SECOND) as u32 % 60);
    let (minutes, hours) = ((clock / MINUTE) as u32 % 60, (clock / HOUR) as u32);
    let (year, month, date) = civil(day);
    if ms != 0 {
        return format!(".{ms:03}");
    }
    if seconds != 0 {
        return format!(":{seconds:02}");
    }
    if minutes != 0 {
        return format!("{:02}:{minutes:02}", twelve(hours));
    }
    if hours != 0 {
        return format!("{:02} {}", twelve(hours), if hours < 12 { "AM" } else { "PM" });
    }
    let name = MONTHS.get(month as usize - 1).copied().unwrap_or_default();
    if date != 1 {
        let weekday = (day + SUNDAY_OFFSET).rem_euclid(7) as usize;
        return if weekday == 0 {
            format!("{} {date:02}", name.get(..3).unwrap_or(name))
        } else {
            format!("{} {date:02}", WEEKDAYS.get(weekday).copied().unwrap_or_default())
        };
    }
    if month != 1 {
        return name.to_owned();
    }
    year.to_string()
}

const fn twelve(hours: u32) -> u32 {
    match hours % 12 {
        0 => 12,
        other => other,
    }
}
