use std::sync::Arc;

use super::PathCommand;

type Pt = [f32; 2];

const SAMPLES: u8 = 16;

#[derive(Clone, Copy)]
enum Piece {
    Line(Pt, Pt),
    Quad(Pt, Pt, Pt),
    Cubic(Pt, Pt, Pt, Pt),
}

fn lerp([x0, y0]: Pt, [x1, y1]: Pt, t: f32) -> Pt {
    [(x1 - x0).mul_add(t, x0), (y1 - y0).mul_add(t, y0)]
}

fn distance([x0, y0]: Pt, [x1, y1]: Pt) -> f32 {
    (x1 - x0).hypot(y1 - y0)
}

impl Piece {
    const fn start(self) -> Pt {
        match self {
            Self::Line(p, _) | Self::Quad(p, _, _) | Self::Cubic(p, _, _, _) => p,
        }
    }

    fn at(self, t: f32) -> Pt {
        match self {
            Self::Line(p0, p1) => lerp(p0, p1, t),
            Self::Quad(p0, p1, p2) => lerp(lerp(p0, p1, t), lerp(p1, p2, t), t),
            Self::Cubic(p0, p1, p2, p3) => {
                let (p01, p12, p23) = (lerp(p0, p1, t), lerp(p1, p2, t), lerp(p2, p3, t));
                lerp(lerp(p01, p12, t), lerp(p12, p23, t), t)
            }
        }
    }

    fn samples(self) -> impl Iterator<Item = (f32, f32)> {
        let mut prev = self.start();
        let mut walked = 0.0;
        (1..=SAMPLES).map(move |step| {
            let t = f32::from(step) / f32::from(SAMPLES);
            let point = self.at(t);
            walked += distance(prev, point);
            prev = point;
            (t, walked)
        })
    }

    fn length(self) -> f32 {
        match self {
            Self::Line(p0, p1) => distance(p0, p1),
            _ => self.samples().last().map_or(0.0, |(_, walked)| walked),
        }
    }

    fn param(self, along: f32) -> f32 {
        if let Self::Line(p0, p1) = self {
            return (along / distance(p0, p1).max(f32::MIN_POSITIVE)).clamp(0.0, 1.0);
        }
        let step = 1.0 / f32::from(SAMPLES);
        let mut before = (0.0, 0.0);
        for (t, walked) in self.samples() {
            if walked >= along {
                let span = (walked - before.1).max(f32::MIN_POSITIVE);
                return ((along - before.1) / span)
                    .mul_add(step, before.0)
                    .clamp(0.0, 1.0);
            }
            before = (t, walked);
        }
        1.0
    }

    fn left(self, t: f32) -> Self {
        match self {
            Self::Line(p0, _) => Self::Line(p0, self.at(t)),
            Self::Quad(p0, p1, _) => Self::Quad(p0, lerp(p0, p1, t), self.at(t)),
            Self::Cubic(p0, p1, p2, _) => {
                let (p01, p12) = (lerp(p0, p1, t), lerp(p1, p2, t));
                Self::Cubic(p0, p01, lerp(p01, p12, t), self.at(t))
            }
        }
    }

    fn right(self, t: f32) -> Self {
        match self {
            Self::Line(_, p1) => Self::Line(self.at(t), p1),
            Self::Quad(_, p1, p2) => Self::Quad(self.at(t), lerp(p1, p2, t), p2),
            Self::Cubic(_, p1, p2, p3) => {
                let (p12, p23) = (lerp(p1, p2, t), lerp(p2, p3, t));
                Self::Cubic(self.at(t), lerp(p12, p23, t), p23, p3)
            }
        }
    }

    fn part(self, t0: f32, t1: f32) -> Self {
        let left = self.left(t1);
        if t0 <= 0.0 || t1 <= 0.0 {
            return left;
        }
        left.right((t0 / t1).clamp(0.0, 1.0))
    }

    const fn command(self) -> PathCommand {
        match self {
            Self::Line(_, [x, y]) => PathCommand::LineTo(x, y),
            Self::Quad(_, [cx, cy], [x, y]) => PathCommand::QuadTo(cx, cy, x, y),
            Self::Cubic(_, [ax, ay], [bx, by], [x, y]) => {
                PathCommand::CubicTo(ax, ay, bx, by, x, y)
            }
        }
    }
}

fn pieces(commands: &[PathCommand], mut visit: impl FnMut(Piece, bool)) {
    let (mut pen, mut first, mut fresh) = ([0.0, 0.0], [0.0, 0.0], true);
    for command in commands {
        let piece = match *command {
            PathCommand::MoveTo(x, y) => {
                (pen, first, fresh) = ([x, y], [x, y], true);
                continue;
            }
            PathCommand::LineTo(x, y) => Piece::Line(pen, [x, y]),
            PathCommand::QuadTo(cx, cy, x, y) => Piece::Quad(pen, [cx, cy], [x, y]),
            PathCommand::CubicTo(ax, ay, bx, by, x, y) => {
                Piece::Cubic(pen, [ax, ay], [bx, by], [x, y])
            }
            PathCommand::Close => Piece::Line(pen, first),
        };
        visit(piece, fresh);
        fresh = false;
        pen = piece.at(1.0);
    }
}

/// The part of `commands` from `start` to `end`, as fractions of the whole outline length.
pub(crate) fn trim(commands: &[PathCommand], [start, end]: [f32; 2]) -> Arc<[PathCommand]> {
    let mut total = 0.0;
    pieces(commands, |piece, _| total += piece.length());
    let (from, to) = (start * total, end * total);
    let mut out = Vec::new();
    let (mut walked, mut open) = (0.0_f32, false);
    pieces(commands, |piece, fresh| {
        open &= !fresh;
        let length = piece.length();
        let (low, high) = (walked, walked + length);
        walked = high;
        if length <= 0.0 || high <= from || low >= to {
            return;
        }
        let t0 = if from > low {
            piece.param(from - low)
        } else {
            0.0
        };
        let t1 = if to < high {
            piece.param(to - low)
        } else {
            1.0
        };
        let part = piece.part(t0, t1);
        if !open || t0 > 0.0 {
            let [x, y] = part.start();
            out.push(PathCommand::MoveTo(x, y));
        }
        out.push(part.command());
        open = true;
    });
    out.into()
}
