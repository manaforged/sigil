use crate::animation::runtime::binding_animate::find_numeric;
use crate::style::atom::{GradientShape, MaskKind, aligned};
use crate::style::{
    ColorAtomKind, Family, NumericAtomKind, RenderStyle, Rgba8, StyleAtom, StyleEngine, Theme, Tone,
};

const FAMILIES: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Shape {
    filters: u16,
    text_shadows: usize,
    gradient: Option<(GradientShape, bool)>,
    mask: Option<MaskKind>,
}

#[derive(Clone, Copy)]
pub(crate) struct Targets {
    pub(crate) numeric: [Option<f32>; NumericAtomKind::COUNT],
    pub(crate) colors: [Option<Rgba8>; ColorAtomKind::COUNT],
    shape: Shape,
}

pub(crate) fn targets(atoms: &[StyleAtom], theme: &Theme, tone: Tone) -> Targets {
    let mut render = RenderStyle::default();
    StyleEngine::with_tone(theme, tone).apply(atoms, &mut render);
    Targets {
        numeric: NumericAtomKind::ALL.map(|kind| numeric_of(&render, atoms, kind)),
        colors: ColorAtomKind::ALL.map(|kind| color_of(&render, kind)),
        shape: Shape {
            filters: render.filter.functions(),
            text_shadows: render.filter.text_count(),
            gradient: render.paint.shape(),
            mask: render.mask.gradient(),
        },
    }
}

fn numeric_of(render: &RenderStyle, atoms: &[StyleAtom], kind: NumericAtomKind) -> Option<f32> {
    match kind {
        NumericAtomKind::Opacity => Some(f32::from(render.opacity)),
        NumericAtomKind::BorderRadius => Some(f32::from(render.border_radius.unwrap_or(0))),
        _ => render
            .transform
            .value(kind)
            .or_else(|| render.space.value(kind))
            .or_else(|| render.filter.value(kind))
            .or_else(|| render.paint.value(kind))
            .or_else(|| render.mask.value(kind))
            .or_else(|| find_numeric(kind, atoms)),
    }
}

fn color_of(render: &RenderStyle, kind: ColorAtomKind) -> Option<Rgba8> {
    match kind {
        ColorAtomKind::Background => render.background_color,
        ColorAtomKind::Text => render.text_color,
        ColorAtomKind::Border => render.border_color,
        ColorAtomKind::DropShadow | ColorAtomKind::TextShadow => render.filter.color(kind),
        ColorAtomKind::StopFrom | ColorAtomKind::StopVia | ColorAtomKind::StopTo => {
            render.paint.color(kind)
        }
    }
}

pub(crate) struct Change {
    pub(crate) from: Targets,
    pub(crate) to: Targets,
    discrete: [bool; FAMILIES],
}

impl Change {
    pub(crate) fn new(from: &Targets, to: &Targets) -> Self {
        let (before, after) = (from.shape, to.shape);
        let discrete = [
            false,
            !aligned(before.filters, after.filters),
            before.text_shadows > 1 || after.text_shadows > 1,
            before.gradient != after.gradient,
            before.mask != after.mask,
        ];
        let mut change = Self {
            from: *from,
            to: *to,
            discrete,
        };
        for kind in NumericAtomKind::ALL {
            let padded = matches!(kind.family(), Family::Filter | Family::TextShadow);
            if padded && !change.discrete(kind.family()) {
                change.pad(kind);
            }
        }
        change
    }

    fn pad(&mut self, kind: NumericAtomKind) {
        let index = kind as usize;
        let (Some(from), Some(to)) = (
            self.from.numeric.get_mut(index),
            self.to.numeric.get_mut(index),
        ) else {
            return;
        };
        match (from.is_some(), to.is_some()) {
            (false, true) => *from = Some(kind.initial()),
            (true, false) => *to = Some(kind.initial()),
            _ => {}
        }
    }

    pub(crate) fn discrete(&self, family: Family) -> bool {
        self.discrete.get(family as usize).copied().unwrap_or(true)
    }

    pub(crate) fn numeric(&self, kind: NumericAtomKind) -> (Option<f32>, Option<f32>) {
        let at = |targets: &Targets| targets.numeric.get(kind as usize).copied().flatten();
        let from = at(&self.from).filter(|_| !self.discrete(kind.family()));
        (from, at(&self.to))
    }

    pub(crate) fn color(&self, kind: ColorAtomKind) -> (Option<Rgba8>, Option<Rgba8>) {
        let at = |targets: &Targets| targets.colors.get(kind as usize).copied().flatten();
        (at(&self.from), at(&self.to))
    }
}
