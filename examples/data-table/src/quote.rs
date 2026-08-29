//! Quote corpus matching GPUI 0.2.2 `examples/data_table.rs`.
//!
//! Same derived change / turnover formulas. Generation is a seeded
//! LCG so the tree is deterministic without a `rand` dependency.

use std::time::Duration;

/// Row count in the GPUI example.
pub const TOTAL_ITEMS: usize = 10_000;

/// Uniform row height in the live window and the 1:1 bench.
pub const ROW_HEIGHT: f32 = 24.0;

/// One stock row. Field set matches GPUI `Quote`.
#[derive(Clone, Debug)]
pub struct Quote {
    pub name: String,
    pub symbol: String,
    pub last_done: f64,
    pub prev_close: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub timestamp: Duration,
    pub volume: u32,
    pub turnover: f64,
    pub ttm: f64,
    pub market_cap: f64,
    pub float_cap: f64,
    pub shares: f64,
    pub pb: f64,
    pub pe: f64,
    pub eps: f64,
    pub dividend: f64,
    pub dividend_yield: f64,
    pub dividend_per_share: f64,
    pub dividend_date: String,
    pub dividend_payment: f64,
}

impl Quote {
    /// Percent change from previous close.
    #[must_use]
    pub fn change(&self) -> f64 {
        (self.last_done - self.prev_close) / self.prev_close * 100.0
    }

    /// True when last done is above previous close.
    #[must_use]
    pub fn up(&self) -> bool {
        self.change() > 0.0
    }

    /// Volume / turnover × 100, matching GPUI `turnover_ratio`.
    #[must_use]
    pub fn turnover_ratio(&self) -> f64 {
        f64::from(self.volume) / self.turnover * 100.0
    }
}

/// Advance last-done on the visible band. In-place; no allocation.
pub fn tick_visible(book: &mut [Quote], first: usize, count: usize, step: u32) {
    let end = first.saturating_add(count).min(book.len());
    let delta = f64::from(step) * 0.001;
    for quote in book.iter_mut().take(end).skip(first) {
        quote.last_done = (quote.last_done + delta).max(0.01);
    }
}

/// Build `count` quotes. Row `i` is seeded from `i`, so the same
/// index always yields the same ticker.
#[must_use]
pub fn generate(count: usize) -> Vec<Quote> {
    (0..count).map(quote_at).collect()
}

#[must_use]
pub fn quote_at(index: usize) -> Quote {
    let mut rng = Rng::new(seed(index));
    fill_quote(&mut rng)
}

const fn seed(index: usize) -> u64 {
    0x9E37_79B9_7F4A_7C15
        ^ (index as u64)
            .wrapping_add(1)
            .wrapping_mul(0xBF58_476D_1CE4_E5B9)
}

fn fill_quote(rng: &mut Rng) -> Quote {
    let prev_close = rng.f64_range(100.0, 200.0);
    let last_done = prev_close + rng.f64_range(-5.0, 5.0);
    let open = prev_close + rng.f64_range(-3.0, 3.0);
    let high = (prev_close + rng.f64_range(0.0, 10.0)).max(open);
    let low = (prev_close - rng.f64_range(0.0, 10.0)).min(open);
    let volume = rng.u32_range(1_000_000, 100_000_000);
    let symbol = ticker(rng);
    let market_cap = rng.f64_range(1_000_000.0, 10_000_000.0);
    let shares = rng.f64_range(100.0, 1000.0);
    let ratio = market_cap / shares;
    Quote {
        name: format!(
            "{} {} - #{}",
            symbol,
            rng.u32_range(1, 100),
            rng.u32_range(10_000, 100_000)
        ),
        symbol,
        last_done,
        prev_close,
        open,
        high,
        low,
        timestamp: Duration::from_secs(u64::from(rng.u32_range(0, 86_400))),
        volume,
        turnover: last_done * f64::from(volume),
        ttm: rng.f64_range(0.0, 10.0),
        market_cap,
        float_cap: market_cap + rng.f64_range(1_000.0, 10_000.0),
        shares,
        pb: ratio,
        pe: ratio,
        eps: ratio,
        dividend: rng.f64_range(0.0, 10.0),
        dividend_yield: rng.f64_range(0.0, 10.0),
        dividend_per_share: rng.f64_range(0.0, 10.0),
        dividend_date: format!(
            "{}-{}-{}",
            rng.u32_range(2000, 2023),
            rng.u32_range(1, 12),
            rng.u32_range(1, 28)
        ),
        dividend_payment: rng.f64_range(0.0, 10.0),
    }
}

fn ticker(rng: &mut Rng) -> String {
    if rng.coin() {
        format!(
            "{:03}.{}.{}",
            rng.u32_range(100, 1000),
            rng.u32_range(0, 10),
            letter(rng)
        )
    } else {
        format!("{}{}.{}", letter(rng), letter(rng), letter(rng))
    }
}

fn letter(rng: &mut Rng) -> char {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "values are bounded to the target range before the cast"
    )]
    char::from(b'A' + rng.u32_range(0, 26) as u8)
}

struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    const fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    const fn coin(&mut self) -> bool {
        self.next_u32() & 1 == 0
    }

    const fn u32_range(&mut self, lo: u32, hi: u32) -> u32 {
        let span = hi.saturating_sub(lo);
        if span == 0 {
            return lo;
        }
        lo.saturating_add(self.next_u32() % span)
    }

    fn f64_range(&mut self, lo: f64, hi: f64) -> f64 {
        let unit = f64::from(self.next_u32()) / f64::from(u32::MAX);
        (hi - lo).mul_add(unit, lo)
    }
}

#[cfg(test)]
#[path = "quote_tests.rs"]
mod tests;
