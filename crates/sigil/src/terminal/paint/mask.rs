use std::ops::Range;

pub(super) type Window = ((usize, usize), (usize, usize));

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Mask {
    windows: Vec<Window>,
    bounds: Window,
    starts: Vec<usize>,
    lines: Vec<Range<usize>>,
}

impl Mask {
    pub(super) fn set(&mut self, windows: impl IntoIterator<Item = Window>) {
        self.windows.clear();
        self.windows.extend(
            windows
                .into_iter()
                .filter(|((x0, y0), (x1, y1))| x0 < x1 && y0 < y1),
        );
        let mut bounds = ((usize::MAX, usize::MAX), (0, 0));
        for ((x0, y0), (x1, y1)) in &self.windows {
            bounds = (
                (bounds.0.0.min(*x0), bounds.0.1.min(*y0)),
                (bounds.1.0.max(*x1), bounds.1.1.max(*y1)),
            );
        }
        self.bounds = if self.windows.is_empty() {
            ((0, 0), (0, 0))
        } else {
            bounds
        };
        self.index();
    }

    fn index(&mut self) {
        self.starts.clear();
        self.lines.clear();
        let mut row = Vec::new();
        for y in self.rows() {
            self.starts.push(self.lines.len());
            row.clear();
            row.extend(
                self.windows
                    .iter()
                    .filter(|((_, y0), (_, y1))| (*y0..*y1).contains(&y))
                    .map(|((x0, _), (x1, _))| *x0..*x1),
            );
            row.sort_unstable_by_key(|span: &Range<usize>| span.start);
            row.dedup_by(|next, open| {
                let joined = next.start <= open.end;
                if joined {
                    open.end = open.end.max(next.end);
                }
                joined
            });
            self.lines.append(&mut row);
        }
        self.starts.push(self.lines.len());
    }

    fn line(&self, y: usize) -> &[Range<usize>] {
        let Some(at) = y.checked_sub(self.bounds.0.1) else {
            return &[];
        };
        match (self.starts.get(at), self.starts.get(at + 1)) {
            (Some(start), Some(end)) => self.lines.get(*start..*end).unwrap_or_default(),
            _ => &[],
        }
    }

    pub(super) fn holds(&self, x: usize, y: usize) -> bool {
        let line = self.line(y);
        let after = line.partition_point(|span| span.start <= x);
        after
            .checked_sub(1)
            .and_then(|at| line.get(at))
            .is_some_and(|span| x < span.end)
    }

    pub(super) fn clamp(&self, xs: Range<usize>, ys: Range<usize>) -> (Range<usize>, Range<usize>) {
        let ((x0, y0), (x1, y1)) = self.bounds;
        (
            xs.start.max(x0)..xs.end.min(x1),
            ys.start.max(y0)..ys.end.min(y1),
        )
    }

    pub(super) const fn rows(&self) -> Range<usize> {
        self.bounds.0.1..self.bounds.1.1
    }

    pub(super) fn spans(&self, y: usize, out: &mut Vec<Range<usize>>) {
        out.clear();
        out.extend_from_slice(self.line(y));
    }
}
