//! Compact access-log book. Both windows allocate the same `Vec`.

#[cfg(feature = "gpui-window")]
use gpui as _;
#[cfg(feature = "sigil-window")]
use sigil as _;

/// Default book size. Override with `ACCESS_LOG_ROWS`.
pub const DEFAULT_ROWS: usize = 1_000_000;
/// Row height. Dense so more cells sit in one frame.
pub const ROW_H: f32 = 16.0;
/// List viewport width.
pub const VIEW_W: f32 = 1248.0;
/// List viewport height. 960 / 16 = 60 visible rows.
pub const VIEW_H: f32 = 960.0;
/// Tick period. Both windows fire this often.
pub const TICK_MS: u64 = 16;
/// Columns painted per row. Same labels in both windows.
pub const HEADERS: [&str; 8] = [
    "time", "method", "path", "status", "bytes", "seq", "cache", "region",
];

/// One access line. 16 bytes. 1M rows ≈ 16 MiB.
#[derive(Clone, Copy, Debug)]
pub struct AccessLine {
    /// Seconds since a fixed origin.
    pub ts: u32,
    /// Response bytes. Bumped on each visible-band tick.
    pub bytes: u32,
    /// Tick count for this line.
    pub seq: u32,
    /// HTTP status.
    pub status: u16,
    /// Method tag.
    pub method: u8,
}

impl AccessLine {
    /// Build line `index`. Deterministic.
    #[must_use]
    pub fn at(index: usize) -> Self {
        let i = u32::try_from(index).unwrap_or(u32::MAX);
        Self {
            ts: 1_700_000_000 + i % 86_400,
            bytes: 200 + (i % 50_000),
            seq: 0,
            // The modulo keeps the index in range; the fallback is unreachable.
            status: STATUSES.get(index % STATUSES.len()).copied().unwrap_or(200),
            method: u8::try_from(index % 4).unwrap_or(0),
        }
    }

    /// One cell. Columns 4 (`bytes`) and 5 (`seq`) change on tick.
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
            // The modulo keeps the index in range; the fallback is unreachable.
            _ => REGION
                .get(index % REGION.len())
                .copied()
                .unwrap_or("iad")
                .to_owned(),
        }
    }

    /// Single-line form for tests.
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

/// Visible row count for this viewport.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
pub fn visible_rows() -> usize {
    (VIEW_H / ROW_H).ceil() as usize
}

/// Overscan on each side. Same as `virtual_list`: one page.
#[must_use]
pub fn overscan_rows() -> usize {
    visible_rows().max(1)
}

/// First geometric (in-view) index at `scroll_y`.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
pub fn first_at(scroll_y: f32) -> usize {
    (scroll_y / ROW_H).floor().max(0.0) as usize
}

/// Mounted window at `scroll_y`: viewport plus one page each side.
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

/// Bump live cells on the visible band.
pub fn tick_visible(book: &mut [AccessLine], first: usize, count: usize) {
    let end = first.saturating_add(count).min(book.len());
    for line in book.iter_mut().take(end).skip(first) {
        line.bytes = line.bytes.wrapping_add(17);
        line.seq = line.seq.wrapping_add(1);
    }
}

/// Allocate `n` lines. This is the book the status line reports.
#[must_use]
pub fn generate(n: usize) -> Vec<AccessLine> {
    (0..n).map(AccessLine::at).collect()
}

/// Book length. `ACCESS_LOG_ROWS` overrides [`DEFAULT_ROWS`].
#[must_use]
pub fn row_count() -> usize {
    std::env::var("ACCESS_LOG_ROWS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_ROWS)
}

/// Cell width. Eight equal columns fill [`VIEW_W`].
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)] // HEADERS.len() is 8 — exactly representable.
pub fn col_w() -> f32 {
    VIEW_W / HEADERS.len() as f32
}

/// Giant HUD line. `us` is the last tick or rebuild.
#[must_use]
pub fn hud_line(engine: &str, us: f64) -> String {
    format!("{engine}  {us:.0} µs")
}

#[cfg(test)]
#[path = "book_tests.rs"]
mod tests;
