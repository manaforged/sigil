pub(crate) const ROW_COUNT: usize = 5_000;

pub(crate) const HEADERS: [&str; 6] = ["id", "ticker", "side", "qty", "price", "venue"];

const COLUMN_WIDTHS: [f32; 6] = [96.0, 96.0, 80.0, 96.0, 128.0, 432.0];

pub(crate) fn column_width(column: usize) -> f32 {
    COLUMN_WIDTHS.get(column).copied().unwrap_or(96.0)
}

#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub(crate) cells: [String; 6],
}

impl Row {
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
