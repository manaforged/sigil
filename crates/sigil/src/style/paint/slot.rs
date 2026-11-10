use crate::kernel::ImageSource;
use crate::style::atom::{MaskAtom, MaskPart, PaintAtom, StopSlot};
use crate::style::{MaskImage, Style, StyleAtom};

mod sealed {
    use crate::style::Style;

    pub struct StyleSlot<'a>(pub(crate) &'a mut Style);

    pub trait PaintTarget: Sized {
        fn style_slot(&mut self) -> StyleSlot<'_>;
    }
}

pub(crate) use sealed::{PaintTarget, StyleSlot};

pub(super) fn push<T: PaintTarget>(mut target: T, atom: StyleAtom) -> T {
    target.style_slot().0.push_atom(atom);
    target
}

pub(super) fn paint<T: PaintTarget>(target: T, atom: PaintAtom) -> T {
    push(target, StyleAtom::Paint(atom))
}

pub(super) fn mask<T: PaintTarget>(target: T, part: MaskPart) -> T {
    push(target, StyleAtom::Mask(MaskAtom(part)))
}

pub(super) fn mask_stops<T: PaintTarget>(target: T, from: u8, to: u8) -> T {
    let target = mask(target, MaskPart::Stop(None, StopSlot::From, from));
    mask(target, MaskPart::Stop(None, StopSlot::To, to))
}

pub(super) fn image<T: PaintTarget>(
    mut target: T,
    source: ImageSource,
    atom: fn(u64) -> StyleAtom,
) -> T {
    let slot = target.style_slot();
    let id = slot.0.push_image(source);
    slot.0.push_atom(atom(id));
    target
}

pub(super) fn mask_image<T: PaintTarget>(target: T, image: MaskImage) -> T {
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

impl PaintTarget for Style {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(self)
    }
}
