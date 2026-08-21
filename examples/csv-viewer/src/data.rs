//! Embedded CSV payload + parser.
//!
//! The corpus is synthesised at startup from a tiny token table so the
//! viewer always has a fixed dataset to virtualise over. Output is
//! deterministic — same row index produces the same six-column row on
//! every run, which keeps the smoke tests pinable without seeding a RNG.

/// Total rows in the synthesised dataset.
pub(crate) const ROW_COUNT: usize = 5_000;

/// Fixed column headers — order matches every [`Row`].
pub(crate) const HEADERS: [&str; 6] = ["id", "ticker", "side", "qty", "price", "venue"];

/// Logical pixel width allocated to each column. The id and ticker
/// columns are widest so the seven-digit ids never wrap; the
/// numerics get medium widths; the venue tail mops up the rest.
const COLUMN_WIDTHS: [f32; 6] = [96.0, 96.0, 80.0, 96.0, 128.0, 432.0];

/// Return the fixed pixel width for `column`, falling back to 96 px
/// when the index is out of range.
pub(crate) fn column_width(column: usize) -> f32 {
    COLUMN_WIDTHS.get(column).copied().unwrap_or(96.0)
}

/// One parsed CSV row, indexed by [`HEADERS`].
#[derive(Clone, Debug)]
pub(crate) struct Row {
    /// Six string cells, one per header.
    pub(crate) cells: [String; 6],
}

impl Row {
    /// Return the cell at `column`, falling back to the empty string if
    /// the index is out of range. Used by the table renderer.
    pub(crate) fn cell(&self, column: usize) -> &str {
        self.cells.get(column).map_or("", String::as_str)
    }
}

const TICKERS: &[&str] = &[
    "AAPL", "MSFT", "GOOG", "AMZN", "NVDA", "META", "TSLA", "AMD",
];
const SIDES: &[&str] = &["BUY", "SELL"];
const VENUES: &[&str] = &["NYSE", "NASDAQ", "ARCA", "BATS", "IEX"];

fn pick<'a>(table: &'a [&'a str], index: usize) -> &'a str {
    table.get(index % table.len()).copied().unwrap_or("?")
}

/// Generate a deterministic CSV corpus of `count` rows.
///
/// Each row carries six string cells: a monotonic id, a rotating ticker,
/// a side, a synthesised quantity, a synthesised price, and a rotating
/// venue. Returned in row order.
pub(crate) fn generate(count: usize) -> Vec<Row> {
    let mut rows = Vec::with_capacity(count);
    for index in 0..count {
        let id = format!("{:06}", index + 1);
        let ticker = pick(TICKERS, index).to_owned();
        let side = pick(SIDES, index / 3).to_owned();
        let qty = ((index * 17) % 980 + 20).to_string();
        let cents = (index * 91) % 10_000;
        let price = format!("{}.{:02}", cents / 100, cents % 100);
        let venue = pick(VENUES, index / 7).to_owned();
        rows.push(Row {
            cells: [id, ticker, side, qty, price, venue],
        });
    }
    rows
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
