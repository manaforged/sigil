use crate::widgets::Date;

use super::super::clock::{DAY_MS, days};

const HOUR_MS: f64 = 3_600_000.0;
const MINUTE_MS: f64 = 60_000.0;
const SECOND_MS: f64 = 1_000.0;
const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];
const MIN_MONTH: usize = 3;

pub(super) fn epoch_ms(text: &str) -> Option<f64> {
    let text = text.trim();
    let parsed = iso(text)
        .or_else(|| text.parse::<f64>().ok())
        .or_else(|| written(text))?;
    parsed.is_finite().then_some(parsed)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a day count from a calendar year is far below f64 precision"
)]
fn day_ms(date: Date) -> f64 {
    days(date) as f64 * DAY_MS
}

fn iso(text: &str) -> Option<f64> {
    let (date, time) = text
        .split_once(['T', ' '])
        .map_or((text, None), |(date, time)| (date, Some(time)));
    let day = match date.len() {
        4 => Date::new(date.parse().ok()?, 1, 1)?,
        7 => Date::parse(&format!("{date}-01"))?,
        _ => Date::parse(date)?,
    };
    Some(day_ms(day) + time.map_or(Some(0.0), clock)?)
}

fn clock(text: &str) -> Option<f64> {
    let (time, offset) = match text.find(['Z', '+', '-']) {
        Some(at) => (text.get(..at)?, zone(text.get(at..)?)?),
        None => (text, 0.0),
    };
    let mut parts = time.split(':');
    let hours = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts
        .next()
        .map_or(Some(0.0), |part| part.parse::<f64>().ok())?;
    let seconds = parts
        .next()
        .map_or(Some(0.0), |part| part.parse::<f64>().ok())?;
    if parts.next().is_some() || hours > 24.0 || minutes >= 60.0 || seconds >= 61.0 {
        return None;
    }
    Some(hours.mul_add(HOUR_MS, minutes.mul_add(MINUTE_MS, seconds * SECOND_MS)) - offset)
}

fn zone(text: &str) -> Option<f64> {
    if text == "Z" {
        return Some(0.0);
    }
    let sign = if text.starts_with('-') { -1.0 } else { 1.0 };
    let digits = text.get(1..)?.replace(':', "");
    let hours = digits.get(..2)?.parse::<f64>().ok()?;
    let minutes = digits.get(2..).filter(|rest| !rest.is_empty());
    let minutes = minutes.map_or(Some(0.0), |rest| rest.parse::<f64>().ok())?;
    Some(sign * hours.mul_add(HOUR_MS, minutes * MINUTE_MS))
}

fn written(text: &str) -> Option<f64> {
    let mut words = text.split([' ', ',']).filter(|word| !word.is_empty());
    let month = month(words.next()?)?;
    let day = words.next()?.parse().ok()?;
    let year = words.next()?.parse().ok()?;
    let time = words.next().map_or(Some(0.0), clock)?;
    if words.next().is_some() {
        return None;
    }
    Some(day_ms(Date::new(year, month, day)?) + time)
}

fn month(word: &str) -> Option<u8> {
    let lower = word.trim_end_matches('.').to_ascii_lowercase();
    let at = MONTHS
        .iter()
        .position(|name| lower.len() >= MIN_MONTH && name.starts_with(&lower))?;
    u8::try_from(at + 1).ok()
}
