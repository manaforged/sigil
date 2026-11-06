use super::{Conic, Radial, RadialExtent, StopSlot};
use crate::kernel::ImageSource;
use crate::style::{AnimatorOverlay, NumericAtomKind};

/// The image whose alpha or luminance shows the node, like CSS `mask-image`.
#[derive(Clone, Debug)]
pub enum MaskImage {
    /// A linear gradient from opaque to clear at `degrees` clockwise from up, like Tailwind `mask-linear-<n>`.
    Linear(i16),
    /// A radial gradient from opaque at the center to clear at its edge, like Tailwind `mask-radial-*`.
    Radial(Radial),
    /// A conic gradient from opaque to clear around its center, like Tailwind `mask-conic-*`.
    Conic(Conic),
    /// An image, like Tailwind `mask-[url(..)]`.
    Image(ImageSource),
}

/// How a mask image is sized against the border box, like CSS `mask-size`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MaskSize {
    /// The image's natural size; gradients fill the box, like Tailwind `mask-auto`.
    #[default]
    Auto,
    /// Scaled to cover the box, like Tailwind `mask-cover`.
    Cover,
    /// Scaled to fit inside the box, like Tailwind `mask-contain`.
    Contain,
}

/// A mask style atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaskAtom(pub(crate) MaskPart);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MaskKind {
    Linear,
    Radial,
    Conic,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MaskPart {
    None,
    Linear(i16),
    Radial(Radial),
    Conic(Conic),
    Image(u64),
    Stop(Option<MaskKind>, StopSlot, u8),
    Circle(bool),
    Extent(RadialExtent),
    RadialAt([u8; 2]),
    Luminance(bool),
    Size(MaskSize),
    Position([u16; 2]),
    Repeat(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MaskStyle {
    pub(crate) kind: Option<MaskKind>,
    pub(crate) angle: i16,
    pub(crate) radial: Radial,
    pub(crate) conic: Conic,
    pub(crate) image: u64,
    pub(crate) stops: [u8; 2],
    pub(crate) luminance: bool,
    pub(crate) size: MaskSize,
    pub(crate) position: [u16; 2],
    pub(crate) repeat: bool,
}

impl MaskStyle {
    pub(crate) const NONE: Self = Self {
        kind: None,
        angle: 0,
        radial: Radial::new(),
        conic: Conic::new(0),
        image: 0,
        stops: [0, 100],
        luminance: false,
        size: MaskSize::Auto,
        position: [0, 0],
        repeat: true,
    };

    pub(crate) fn apply(&mut self, part: MaskPart) {
        match part {
            MaskPart::None => self.kind = None,
            MaskPart::Linear(degrees) => {
                self.kind = Some(MaskKind::Linear);
                self.angle = degrees;
            }
            MaskPart::Radial(radial) => {
                self.kind = Some(MaskKind::Radial);
                self.radial = radial;
            }
            MaskPart::Conic(conic) => {
                self.kind = Some(MaskKind::Conic);
                self.conic = conic;
            }
            MaskPart::Image(id) => {
                self.kind = Some(MaskKind::Image);
                self.image = id;
            }
            MaskPart::Stop(kind, slot, percent) => self.stop(kind, slot, percent),
            MaskPart::Circle(circle) => self.radial.circle = circle,
            MaskPart::Extent(extent) => self.radial.extent = extent,
            MaskPart::RadialAt(at) => self.radial.at = at,
            MaskPart::Luminance(luminance) => self.luminance = luminance,
            MaskPart::Size(size) => self.size = size,
            MaskPart::Position(position) => self.position = position,
            MaskPart::Repeat(repeat) => self.repeat = repeat,
        }
    }

    fn stop(&mut self, kind: Option<MaskKind>, slot: StopSlot, percent: u8) {
        if kind.is_some() {
            self.kind = kind;
        }
        let percent = if percent > 100 { 100 } else { percent };
        match slot {
            StopSlot::From | StopSlot::Via => self.stops[0] = percent,
            StopSlot::To => self.stops[1] = percent,
        }
    }
}

impl MaskStyle {
    pub(crate) const fn gradient(&self) -> Option<MaskKind> {
        match self.kind {
            Some(MaskKind::Image) | None => None,
            kind => kind,
        }
    }

    pub(crate) fn value(&self, kind: NumericAtomKind) -> Option<f32> {
        let index = match kind {
            NumericAtomKind::MaskFrom => 0,
            NumericAtomKind::MaskTo => 1,
            _ => return None,
        };
        let stop = self.stops.get(index).copied()?;
        self.gradient().map(|_| f32::from(stop))
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the percent is clamped to 0..=100 before the cast"
    )]
    pub(crate) fn overlay(&mut self, overlay: &dyn AnimatorOverlay) {
        if self.gradient().is_none() {
            return;
        }
        for (slot, kind) in self
            .stops
            .iter_mut()
            .zip([NumericAtomKind::MaskFrom, NumericAtomKind::MaskTo])
        {
            if let Some(percent) = overlay.value_for(kind) {
                *slot = percent.round().clamp(0.0, 100.0) as u8;
            }
        }
    }
}

impl Default for MaskStyle {
    fn default() -> Self {
        Self::NONE
    }
}
