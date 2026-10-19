pub(super) const UP: u8 = 1;
pub(super) const DOWN: u8 = 2;
pub(super) const LEFT: u8 = 4;
pub(super) const RIGHT: u8 = 8;
const MAX_COLS: usize = 1024;
const MAX_AREA: usize = 1024 * 512;
const LABEL_SHIFTS: [usize; 4] = [0, 1, 2, 3];

pub(super) struct Canvas {
    cols: usize,
    rows: usize,
    cells: Vec<char>,
    labels: Vec<(usize, usize, String)>,
}

impl Canvas {
    pub(super) fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.min(MAX_COLS);
        let rows = rows.min(MAX_AREA / cols.max(1));
        Self {
            cols,
            rows,
            cells: vec![' '; cols.saturating_mul(rows)],
            labels: Vec::new(),
        }
    }

    fn slot(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.cols && y < self.rows).then(|| y.saturating_mul(self.cols).saturating_add(x))
    }

    pub(super) fn put(&mut self, x: usize, y: usize, ch: char) {
        if let Some(cell) = self.slot(x, y).and_then(|index| self.cells.get_mut(index)) {
            *cell = join(*cell, ch);
        }
    }

    pub(super) fn set(&mut self, x: usize, y: usize, ch: char) {
        if let Some(cell) = self.slot(x, y).and_then(|index| self.cells.get_mut(index)) {
            *cell = ch;
        }
    }

    pub(super) fn text(&mut self, x: usize, y: usize, text: &str) {
        for (offset, ch) in text.chars().enumerate() {
            self.set(x.saturating_add(offset), y, ch);
        }
    }

    pub(super) fn label(&mut self, x: usize, y: usize, text: &str) {
        self.labels.push((x, y, text.to_string()));
    }

    pub(super) fn hline(&mut self, from: usize, to: usize, y: usize, ch: char) {
        for x in from.min(to)..=from.max(to) {
            self.put(x, y, ch);
        }
    }

    pub(super) fn vline(&mut self, x: usize, from: usize, to: usize, ch: char) {
        for y in from.min(to)..=from.max(to) {
            self.put(x, y, ch);
        }
    }

    pub(super) fn corner(&mut self, x: usize, y: usize, bits: u8) {
        self.put(x, y, glyph(bits));
    }

    pub(super) fn frame(&mut self, x: usize, y: usize, size: (usize, usize), round: bool) {
        let (width, height) = size;
        if width < 2 || height < 2 {
            return;
        }
        let right = x.saturating_add(width).saturating_sub(1);
        let bottom = y.saturating_add(height).saturating_sub(1);
        let blank = " ".repeat(width);
        for row in y..=bottom {
            self.text(x, row, &blank);
        }
        self.hline(x, right, y, '─');
        self.hline(x, right, bottom, '─');
        self.vline(x, y, bottom, '│');
        self.vline(right, y, bottom, '│');
        let (tl, tr, bl, br) = if round {
            ('╭', '╮', '╰', '╯')
        } else {
            ('┌', '┐', '└', '┘')
        };
        self.set(x, y, tl);
        self.set(right, y, tr);
        self.set(x, bottom, bl);
        self.set(right, bottom, br);
    }

    pub(super) fn lines(&self) -> Vec<String> {
        let mut cells = self.cells.clone();
        let mut taken = vec![false; cells.len()];
        for (x, y, text) in &self.labels {
            let span = |row: usize| {
                (0..text.chars().count())
                    .filter_map(move |offset| self.slot(x.saturating_add(offset), row))
            };
            let row = LABEL_SHIFTS
                .iter()
                .map(|shift| y.saturating_add(*shift))
                .find(|row| span(*row).all(|index| !taken.get(index).copied().unwrap_or(true)))
                .unwrap_or(*y);
            for (index, ch) in span(row).zip(text.chars()) {
                if let (Some(cell), Some(used)) = (cells.get_mut(index), taken.get_mut(index)) {
                    *cell = ch;
                    *used = true;
                }
            }
        }
        cells
            .chunks(self.cols.max(1))
            .take(self.rows)
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect()
    }
}

pub(super) const fn bits(ch: char) -> Option<u8> {
    match ch {
        '─' | '┄' => Some(LEFT | RIGHT),
        '│' | '┆' => Some(UP | DOWN),
        '┌' | '╭' => Some(DOWN | RIGHT),
        '┐' | '╮' => Some(DOWN | LEFT),
        '└' | '╰' => Some(UP | RIGHT),
        '┘' | '╯' => Some(UP | LEFT),
        '├' => Some(UP | DOWN | RIGHT),
        '┤' => Some(UP | DOWN | LEFT),
        '┬' => Some(DOWN | LEFT | RIGHT),
        '┴' => Some(UP | LEFT | RIGHT),
        '┼' => Some(UP | DOWN | LEFT | RIGHT),
        _ => None,
    }
}

pub(super) const fn glyph(bits: u8) -> char {
    match bits {
        0b0001..=0b0011 => '│',
        0b1100 | 0b0100 | 0b1000 => '─',
        0b1010 => '┌',
        0b0110 => '┐',
        0b1001 => '└',
        0b0101 => '┘',
        0b1011 => '├',
        0b0111 => '┤',
        0b1110 => '┬',
        0b1101 => '┴',
        _ => '┼',
    }
}

const fn join(old: char, new: char) -> char {
    match (bits(old), bits(new)) {
        (Some(a), Some(b)) => glyph(a | b),
        _ => new,
    }
}
