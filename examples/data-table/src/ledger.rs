use std::cmp::Ordering;

use crate::quote::Quote;
use crate::{FieldKind, cell_text};

pub const MIXED_EVERY: usize = 50;
pub const UNDO_DEPTH: usize = 100;
pub const TICK_STRIDE: usize = 40;

const MIXED_NAMES: [&str; 8] = [
    "東京証券 株式会社",
    "مصرف الخليج الأول",
    "בנק הפועלים",
    "🚀 Rocket Labs ⚡",
    "서울반도체",
    "北京能源集团",
    "Zürich Rück AG",
    "ΑΘΗΝΑ Ενέργεια",
];

#[derive(Clone, Debug)]
pub struct Row {
    pub index: usize,
    pub quote: Quote,
}

impl Row {
    #[must_use]
    pub fn text(&self, kind: FieldKind) -> String {
        cell_text(self.index, &self.quote, kind)
    }

    #[must_use]
    pub fn number(&self, kind: FieldKind) -> Option<f64> {
        let q = &self.quote;
        Some(match kind {
            FieldKind::PrevClose => q.prev_close,
            FieldKind::Open => q.open,
            FieldKind::Low => q.low,
            FieldKind::High => q.high,
            FieldKind::Ttm => q.ttm,
            FieldKind::MarketCap => q.market_cap,
            FieldKind::FloatCap => q.float_cap,
            FieldKind::Turnover => q.turnover,
            FieldKind::Volume => f64::from(q.volume),
            FieldKind::TurnoverRatio => q.turnover_ratio(),
            FieldKind::Pe => q.pe,
            FieldKind::Pb => q.pb,
            FieldKind::Eps => q.eps,
            FieldKind::Shares => q.shares,
            FieldKind::Dividend => q.dividend,
            FieldKind::Yield => q.dividend_yield,
            FieldKind::DividendPerShare => q.dividend_per_share,
            FieldKind::DividendPayment => q.dividend_payment,
            FieldKind::Timestamp => q.timestamp.as_secs_f64(),
            FieldKind::Id
            | FieldKind::Symbol
            | FieldKind::Name
            | FieldKind::DividendDate
            | FieldKind::LastDone => return None,
        })
    }

    #[must_use]
    pub fn matches(&self, needle: &str) -> bool {
        let needle = needle.to_lowercase();
        self.quote.symbol.to_lowercase().contains(&needle)
            || self.quote.name.to_lowercase().contains(&needle)
    }
}

#[must_use]
pub fn compare(a: &Row, b: &Row, kind: FieldKind) -> Ordering {
    match kind {
        FieldKind::Id => a.index.cmp(&b.index),
        FieldKind::Symbol => a.quote.symbol.cmp(&b.quote.symbol),
        FieldKind::Name => a.quote.name.cmp(&b.quote.name),
        FieldKind::DividendDate => a.quote.dividend_date.cmp(&b.quote.dividend_date),
        numeric => match (a.number(numeric), b.number(numeric)) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            _ => Ordering::Equal,
        },
    }
}

#[must_use]
pub fn generate(count: usize) -> Vec<Row> {
    crate::generate(count)
        .into_iter()
        .enumerate()
        .map(|(index, mut quote)| {
            if index.is_multiple_of(MIXED_EVERY) {
                let name = MIXED_NAMES
                    .get((index / MIXED_EVERY) % MIXED_NAMES.len())
                    .copied()
                    .unwrap_or_default();
                quote.name = format!("{name} · {}", quote.symbol);
            }
            Row { index, quote }
        })
        .collect()
}

#[must_use]
pub fn prices(book: &[Row]) -> Vec<f64> {
    book.iter().map(|row| row.quote.last_done).collect()
}

pub fn tick(prices: &mut [f64], step: u64) -> usize {
    let phase = usize::try_from(step).unwrap_or(0) % TICK_STRIDE;
    let delta = if step.is_multiple_of(2) { 0.05 } else { -0.03 };
    let mut moved = 0;
    for price in prices.iter_mut().skip(phase).step_by(TICK_STRIDE) {
        *price = (*price + delta).max(0.01);
        moved += 1;
    }
    moved
}

#[must_use]
pub fn parse_price(text: &str) -> Option<f64> {
    let value: f64 = text.trim().parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;
