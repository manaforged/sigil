use crate::kernel::ImageSource;
use crate::reactive::Reactive;
use crate::style::atom::{MaskAtom, MaskPart, PaintAtom, StopSlot};
use crate::style::{MaskImage, Paint, StyleAtom};

pub(super) fn push<T: Paint>(mut target: T, atom: StyleAtom) -> T {
    target.paint_mut().push_atom(atom);
    target
}

pub(super) fn live<T, V, A>(mut target: T, value: impl Reactive<V>, atoms: fn(V) -> A) -> T
where
    T: Paint,
    V: 'static,
    A: IntoIterator<Item = StyleAtom> + 'static,
{
    let now = value.try_static().map(atoms);
    target
        .paint_mut()
        .push_live(now, move || atoms(value.read()));
    target
}

pub(super) fn paint<T: Paint>(target: T, atom: PaintAtom) -> T {
    push(target, StyleAtom::Paint(atom))
}

pub(super) fn mask<T: Paint>(target: T, part: MaskPart) -> T {
    push(target, StyleAtom::Mask(MaskAtom(part)))
}

pub(super) fn mask_stops<T: Paint>(target: T, from: u8, to: u8) -> T {
    let target = mask(target, MaskPart::Stop(None, StopSlot::From, from));
    mask(target, MaskPart::Stop(None, StopSlot::To, to))
}

pub(super) fn image<T: Paint>(mut target: T, source: ImageSource, atom: fn(u64) -> StyleAtom) -> T {
    let slot = target.paint_mut();
    let id = slot.push_image(source);
    slot.push_atom(atom(id));
    target
}

pub(super) fn mask_image<T: Paint>(target: T, image: MaskImage) -> T {
    let part = match image {
        MaskImage::Linear(degrees) => MaskPart::Linear(degrees),
        MaskImage::Radial(radial) => MaskPart::Radial(radial),
        MaskImage::Conic(conic) => MaskPart::Conic(conic),
        MaskImage::Image(source) => {
            return self::image(target, source, |id| {
                StyleAtom::Mask(MaskAtom(MaskPart::Image(id)))
            });
        }
    };
    mask(target, part)
}
