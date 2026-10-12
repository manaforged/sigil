use super::canvas::{Canvas, DOWN, LEFT, RIGHT, UP};
use super::flow::{Dir, Edge, Rect};

pub(super) struct Pen {
    pub(super) dir: Dir,
    pub(super) dashed: bool,
}

impl Pen {
    pub(super) const fn split(&self, point: (usize, usize)) -> (usize, usize) {
        if self.dir.vertical() {
            (point.1, point.0)
        } else {
            (point.0, point.1)
        }
    }

    pub(super) const fn point(&self, primary: usize, secondary: usize) -> (usize, usize) {
        if self.dir.vertical() {
            (secondary, primary)
        } else {
            (primary, secondary)
        }
    }

    pub(super) const fn glyphs(&self) -> (char, char) {
        match (self.dir.vertical(), self.dashed) {
            (true, false) => ('│', '─'),
            (true, true) => ('┆', '┄'),
            (false, false) => ('─', '│'),
            (false, true) => ('┄', '┆'),
        }
    }

    pub(super) fn primary(&self, canvas: &mut Canvas, secondary: usize, from: usize, to: usize) {
        let (glyph, _) = self.glyphs();
        if self.dir.vertical() {
            canvas.vline(secondary, from, to, glyph);
        } else {
            canvas.hline(from, to, secondary, glyph);
        }
    }

    pub(super) fn secondary(&self, canvas: &mut Canvas, primary: usize, from: usize, to: usize) {
        let (_, glyph) = self.glyphs();
        if self.dir.vertical() {
            canvas.hline(from, to, primary, glyph);
        } else {
            canvas.vline(primary, from, to, glyph);
        }
    }

    pub(super) const fn back(&self) -> u8 {
        match self.dir {
            Dir::Down => UP,
            Dir::Up => DOWN,
            Dir::Right => LEFT,
            Dir::Left => RIGHT,
        }
    }

    pub(super) const fn ahead(&self) -> u8 {
        match self.dir {
            Dir::Down => DOWN,
            Dir::Up => UP,
            Dir::Right => RIGHT,
            Dir::Left => LEFT,
        }
    }

    pub(super) const fn sideways(&self, from: usize, to: usize) -> (u8, u8) {
        match (self.dir.vertical(), to > from) {
            (true, true) => (RIGHT, LEFT),
            (true, false) => (LEFT, RIGHT),
            (false, true) => (DOWN, UP),
            (false, false) => (UP, DOWN),
        }
    }

    pub(super) const fn arrow(&self) -> char {
        match self.dir {
            Dir::Down => '▼',
            Dir::Up => '▲',
            Dir::Right => '▶',
            Dir::Left => '◀',
        }
    }
}

pub(super) const fn exit(rect: Rect, dir: Dir) -> (usize, usize) {
    match dir {
        Dir::Down => (rect.x + rect.w / 2, rect.y + rect.h),
        Dir::Up => (rect.x + rect.w / 2, rect.y.saturating_sub(1)),
        Dir::Right => (rect.x + rect.w, rect.y + rect.h / 2),
        Dir::Left => (rect.x.saturating_sub(1), rect.y + rect.h / 2),
    }
}

pub(super) const fn entry(rect: Rect, dir: Dir) -> (usize, usize) {
    match dir {
        Dir::Down => (rect.x + rect.w / 2, rect.y.saturating_sub(1)),
        Dir::Up => (rect.x + rect.w / 2, rect.y + rect.h),
        Dir::Right => (rect.x.saturating_sub(1), rect.y + rect.h / 2),
        Dir::Left => (rect.x + rect.w, rect.y + rect.h / 2),
    }
}

pub(super) fn span(from: usize, to: usize) -> Option<(usize, usize)> {
    match from.cmp(&to) {
        std::cmp::Ordering::Less => Some((from, to.saturating_sub(1))),
        std::cmp::Ordering::Greater => Some((to.saturating_add(1), from)),
        std::cmp::Ordering::Equal => None,
    }
}

pub(super) fn between(from: usize, to: usize) -> Option<(usize, usize)> {
    let (low, high) = (from.min(to), from.max(to));
    (high.saturating_sub(low) > 1).then(|| (low.saturating_add(1), high.saturating_sub(1)))
}

pub(super) fn route(canvas: &mut Canvas, edge: &Edge, from: Rect, to: Rect, dir: Dir) {
    let pen = Pen {
        dir,
        dashed: edge.dashed,
    };
    let (pp, ps) = pen.split(exit(from, dir));
    let (qp, qs) = pen.split(entry(to, dir));
    let head = if edge.arrow {
        pen.arrow()
    } else {
        pen.glyphs().0
    };
    if ps == qs {
        if let Some((low, high)) = span(pp, qp) {
            pen.primary(canvas, ps, low, high);
        }
        let (x5, y5) = pen.point(qp, qs);
        canvas.set(x5, y5, head);
        label(canvas, &pen, edge, (pp, ps), pp.saturating_add(qp) / 2);
        return;
    }
    jog(canvas, &pen, edge, [(pp, ps), (qp, qs)], head);
}

fn jog(canvas: &mut Canvas, pen: &Pen, edge: &Edge, ends: [(usize, usize); 2], head: char) {
    let [(pp, ps), (qp, qs)] = ends;
    let mid = pp.saturating_add(qp) / 2;
    if let Some((low, high)) = span(pp, mid) {
        pen.primary(canvas, ps, low, high);
    }
    let (toward, back) = pen.sideways(ps, qs);
    let (x6, y6) = pen.point(mid, ps);
    canvas.corner(x6, y6, pen.back() | toward);
    if let Some((low, high)) = between(ps, qs) {
        pen.secondary(canvas, mid, low, high);
    }
    let (x7, y7) = pen.point(mid, qs);
    canvas.corner(x7, y7, back | pen.ahead());
    if let Some((low, high)) = between(mid, qp) {
        pen.primary(canvas, qs, low, high);
    }
    let (x8, y8) = pen.point(qp, qs);
    canvas.set(x8, y8, head);
    label(canvas, pen, edge, (pp, ps), pp);
}

fn label(canvas: &mut Canvas, pen: &Pen, edge: &Edge, start: (usize, usize), along: usize) {
    if edge.label.is_empty() {
        return;
    }
    let (pp, ps) = start;
    let (x, y) = if pen.dir.vertical() {
        pen.point(along, ps.saturating_add(2))
    } else {
        pen.point(pp.saturating_add(1), ps.saturating_sub(1))
    };
    canvas.text(x, y, &edge.label);
}
