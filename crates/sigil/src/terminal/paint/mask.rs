use std::ops::Range;

pub(super) type Window = ((usize, usize), (usize, usize));

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Mask {
    windows: Vec<Window>,
    bounds: Window,
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
    }

    pub(super) fn holds(&self, x: usize, y: usize) -> bool {
        self.windows
            .iter()
            .any(|((x0, y0), (x1, y1))| (*x0..*x1).contains(&x) && (*y0..*y1).contains(&y))
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
        out.extend(
            self.windows
                .iter()
                .filter(|((_, y0), (_, y1))| (*y0..*y1).contains(&y))
                .map(|((x0, _), (x1, _))| *x0..*x1),
        );
        out.sort_unstable_by_key(|span| span.start);
        out.dedup_by(|next, open| {
            let joined = next.start <= open.end;
            if joined {
                open.end = open.end.max(next.end);
            }
            joined
        });
    }
}
