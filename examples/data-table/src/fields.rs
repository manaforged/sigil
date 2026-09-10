use crate::quote::Quote;

pub const COLUMN_COUNT: usize = 24;

pub const TABLE_WIDTH: f32 = 1248.0;

pub const VIEWPORT_HEIGHT: f32 = 912.0;

#[derive(Clone, Copy)]
pub struct Field {
    pub key: &'static str,
    pub width: f32,
    pub kind: FieldKind,
    pub tint: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Id,
    Symbol,
    Name,
    LastDone,
    PrevClose,
    Open,
    Low,
    High,
    Ttm,
    MarketCap,
    FloatCap,
    Turnover,
    Volume,
    TurnoverRatio,
    Pe,
    Pb,
    Eps,
    Shares,
    Dividend,
    Yield,
    DividendPerShare,
    DividendDate,
    DividendPayment,
    Timestamp,
}

impl FieldKind {
    #[must_use]
    pub const fn live(self) -> bool {
        matches!(self, Self::LastDone)
    }
}

#[must_use]
pub fn columns_intersecting(max_x: f32) -> usize {
    let mut x = 0.0;
    let mut n = 0;
    for field in FIELDS {
        if x < max_x {
            n += 1;
        }
        x += field.width;
    }
    n
}

const fn col(key: &'static str, width: f32, kind: FieldKind, tint: bool) -> Field {
    Field {
        key,
        width,
        kind,
        tint,
    }
}

pub const FIELDS: [Field; COLUMN_COUNT] = [
    col("id", 64.0, FieldKind::Id, false),
    col("symbol", 64.0, FieldKind::Symbol, false),
    col("name", 180.0, FieldKind::Name, false),
    col("last_done", 80.0, FieldKind::LastDone, true),
    col("prev_close", 80.0, FieldKind::PrevClose, true),
    col("open", 80.0, FieldKind::Open, true),
    col("low", 80.0, FieldKind::Low, true),
    col("high", 80.0, FieldKind::High, true),
    col("ttm", 50.0, FieldKind::Ttm, true),
    col("market_cap", 96.0, FieldKind::MarketCap, false),
    col("float_cap", 96.0, FieldKind::FloatCap, false),
    col("turnover", 120.0, FieldKind::Turnover, false),
    col("volume", 100.0, FieldKind::Volume, false),
    col("turnover_ratio", 96.0, FieldKind::TurnoverRatio, false),
    col("pe", 64.0, FieldKind::Pe, false),
    col("pb", 64.0, FieldKind::Pb, false),
    col("eps", 64.0, FieldKind::Eps, true),
    col("shares", 96.0, FieldKind::Shares, false),
    col("dividend", 64.0, FieldKind::Dividend, false),
    col("yield", 64.0, FieldKind::Yield, false),
    col(
        "dividend_per_share",
        64.0,
        FieldKind::DividendPerShare,
        false,
    ),
    col("dividend_date", 96.0, FieldKind::DividendDate, false),
    col("dividend_payment", 64.0, FieldKind::DividendPayment, false),
    col("timestamp", 120.0, FieldKind::Timestamp, true),
];

#[must_use]
pub fn header_label(key: &str) -> String {
    key.replace('_', " ").to_uppercase()
}

#[must_use]
pub fn cell_text(index: usize, quote: &Quote, kind: FieldKind) -> String {
    match kind {
        FieldKind::Id => format!("{index}"),
        FieldKind::Symbol => quote.symbol.clone(),
        FieldKind::Name => quote.name.clone(),
        FieldKind::DividendDate => quote.dividend_date.clone(),
        FieldKind::Timestamp => format!("{}", quote.timestamp.as_secs()),
        other => numeric_text(quote, other),
    }
}

fn numeric_text(quote: &Quote, kind: FieldKind) -> String {
    match kind {
        FieldKind::LastDone => format!("{:.3}", quote.last_done),
        FieldKind::PrevClose => format!("{:.3}", quote.prev_close),
        FieldKind::Open => format!("{:.2}", quote.open),
        FieldKind::Low => format!("{:.2}", quote.low),
        FieldKind::High => format!("{:.2}", quote.high),
        FieldKind::Ttm => format!("{:.2}", quote.ttm),
        FieldKind::MarketCap => millions(quote.market_cap),
        FieldKind::FloatCap => millions(quote.float_cap),
        FieldKind::Turnover => millions(quote.turnover),
        FieldKind::Volume => millions(f64::from(quote.volume)),
        FieldKind::TurnoverRatio => format!("{:.2}%", quote.turnover_ratio()),
        FieldKind::Pe => format!("{:.2}", quote.pe),
        FieldKind::Pb => format!("{:.2}", quote.pb),
        FieldKind::Eps => format!("{:.2}", quote.eps),
        FieldKind::Shares => format!("{:.2}", quote.shares),
        FieldKind::Dividend => format!("{:.2}", quote.dividend),
        FieldKind::Yield => format!("{:.2}%", quote.dividend_yield),
        FieldKind::DividendPerShare => format!("{:.2}", quote.dividend_per_share),
        FieldKind::DividendPayment => format!("{:.2}", quote.dividend_payment),
        _ => String::from("--"),
    }
}

fn millions(value: f64) -> String {
    format!("{:.2} M", value / 1_000_000.0)
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod tests;
