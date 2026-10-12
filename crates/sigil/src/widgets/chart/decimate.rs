#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Candle {
    pub(super) open: f32,
    pub(super) high: f32,
    pub(super) low: f32,
    pub(super) close: f32,
}

impl Candle {
    pub(super) fn rows(flat: &[f32]) -> Vec<Self> {
        flat.as_chunks::<4>()
            .0
            .iter()
            .map(|&[open, high, low, close]| Self {
                open,
                high,
                low,
                close,
            })
            .collect()
    }

    const fn merge(self, next: Self) -> Self {
        Self {
            open: self.open,
            high: self.high.max(next.high),
            low: self.low.min(next.low),
            close: next.close,
        }
    }
}

pub(super) fn min_max(values: &[f32], columns: usize) -> Vec<(usize, f32)> {
    let columns = columns.max(1);
    if values.len() <= columns.saturating_mul(2) {
        return values.iter().copied().enumerate().collect();
    }
    let mut points = Vec::with_capacity(columns * 2);
    for column in 0..columns {
        let start = column * values.len() / columns;
        let end = ((column + 1) * values.len() / columns).max(start + 1);
        if let Some((low, high)) = extremes(values, start, end) {
            let (first, last) = if low.0 <= high.0 {
                (low, high)
            } else {
                (high, low)
            };
            points.push(first);
            if low.0 != high.0 {
                points.push(last);
            }
        }
    }
    points
}

type Extremes = ((usize, f32), (usize, f32));

fn extremes(values: &[f32], start: usize, end: usize) -> Option<Extremes> {
    let bucket = values.get(start..end)?;
    let mut points = bucket.iter().copied().enumerate();
    let (first, initial) = points.find(|(_, value)| value.is_finite())?;
    let seed = (start + first, initial);
    Some(points.filter(|(_, value)| value.is_finite()).fold(
        (seed, seed),
        |(low, high), (offset, value)| {
            let point = (start + offset, value);
            (
                if value < low.1 { point } else { low },
                if value > high.1 { point } else { high },
            )
        },
    ))
}

pub(super) fn candles(rows: &[Candle], columns: usize) -> Vec<Candle> {
    let columns = columns.max(1);
    if rows.len() <= columns {
        return rows.to_vec();
    }
    (0..columns)
        .filter_map(|column| {
            let start = column * rows.len() / columns;
            let end = ((column + 1) * rows.len() / columns).max(start + 1);
            let bucket = rows.get(start..end)?;
            let (first, rest) = bucket.split_first()?;
            Some(rest.iter().fold(*first, |acc, row| acc.merge(*row)))
        })
        .collect()
}
