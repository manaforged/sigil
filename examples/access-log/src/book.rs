#[cfg(feature = "gpui-window")]
use gpui as _;
#[cfg(feature = "sigil-window")]
use sigil as _;

pub const DEFAULT_ROWS: usize = 1_000_000;
pub const ROW_H: f32 = 16.0;
pub const VIEW_W: f32 = 1248.0;
pub const VIEW_H: f32 = 960.0;
pub const TICK_MS: u64 = 16;
pub const HEADERS: [&str; 8] = [
    "time", "method", "path", "status", "bytes", "seq", "cache", "region",
];

#[derive(Clone, Copy, Debug)]
pub struct AccessLine {
    pub ts: u32,
    pub bytes: u32,
    pub seq: u32,
    pub status: u16,
    pub method: u8,
}

impl AccessLine {
    #[must_use]
    pub fn at(index: usize) -> Self {
        let i = u32::try_from(index).unwrap_or(u32::MAX);
        Self {
            ts: 1_700_000_000 + i % 86_400,
            bytes: 200 + (i % 50_000),
            seq: 0,
            status: STATUSES.get(index % STATUSES.len()).copied().unwrap_or(200),
            method: u8::try_from(index % 4).unwrap_or(0),
        }
    }

    #[must_use]
    pub fn cell(self, index: usize, col: usize) -> String {
        match col {
            0 => clock(self.ts),
            1 => method_label(self.method).to_owned(),
            2 => format!("/api/v1/items/{}", index % 10_000),
            3 => self.status.to_string(),
            4 => self.bytes.to_string(),
            5 => self.seq.to_string(),
            6 => if self.status == 200 { "HIT" } else { "MISS" }.to_owned(),
            _ => REGION
                .get(index % REGION.len())
                .copied()
                .unwrap_or("iad")
                .to_owned(),
        }
    }

    #[must_use]
    pub fn format(self, index: usize) -> String {
        format!(
            "{} {} {} {} {}",
            self.cell(index, 0),
            self.cell(index, 1),
            self.cell(index, 2),
            self.cell(index, 3),
            self.cell(index, 4),
        )
    }
}

const STATUSES: [u16; 6] = [200, 200, 201, 304, 404, 500];
const REGION: [&str; 4] = ["iad", "sfo", "lhr", "nrt"];

const fn method_label(tag: u8) -> &'static str {
    match tag {
        0 => "GET",
        1 => "POST",
        2 => "PUT",
        _ => "DELETE",
    }
}

fn clock(ts: u32) -> String {
    let sec = ts % 86_400;
    let h = sec / 3600;
    let m = (sec % 3600) / 60;
    let s = sec % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
pub fn visible_rows() -> usize {
    (VIEW_H / ROW_H).ceil() as usize
}

#[must_use]
pub fn overscan_rows() -> usize {
    visible_rows().max(1)
}

#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
pub fn first_at(scroll_y: f32) -> usize {
    (scroll_y / ROW_H).floor().max(0.0) as usize
}

#[must_use]
pub fn window_at(scroll_y: f32, total: usize) -> (usize, usize) {
    let pad = overscan_rows();
    let vis = visible_rows();
    let first = first_at(scroll_y).saturating_sub(pad).min(total);
    let end = first_at(scroll_y)
        .saturating_add(vis)
        .saturating_add(pad)
        .min(total);
    (first, end)
}

pub fn tick_visible(book: &mut [AccessLine], first: usize, count: usize) {
    let end = first.saturating_add(count).min(book.len());
    for line in book.iter_mut().take(end).skip(first) {
        line.bytes = line.bytes.wrapping_add(17);
        line.seq = line.seq.wrapping_add(1);
    }
}

#[must_use]
pub fn generate(n: usize) -> Vec<AccessLine> {
    (0..n).map(AccessLine::at).collect()
}

#[must_use]
pub fn row_count() -> usize {
    std::env::var("ACCESS_LOG_ROWS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_ROWS)
}

#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
pub fn col_w() -> f32 {
    VIEW_W / HEADERS.len() as f32
}

#[must_use]
pub fn hud_line(engine: &str, us: f64) -> String {
    format!("{engine}  {us:.0} µs")
}

#[cfg(test)]
#[path = "book_tests.rs"]
mod tests;
