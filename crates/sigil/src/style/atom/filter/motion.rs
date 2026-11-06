use super::{Amount, FilterStyle, Rgba8, ShadowSpec, TextShadows, blur_units, hue_units, length};
use crate::style::{AnimatorOverlay, ColorAtomKind, NumericAtomKind as K};

const AMOUNTS: [(Amount, K); 6] = [
    (Amount::Brightness, K::Brightness),
    (Amount::Contrast, K::Contrast),
    (Amount::Grayscale, K::Grayscale),
    (Amount::Invert, K::Invert),
    (Amount::Saturate, K::Saturate),
    (Amount::Sepia, K::Sepia),
];

const ORDER: [K; 9] = [
    K::Blur,
    K::Brightness,
    K::Contrast,
    K::Grayscale,
    K::HueRotate,
    K::Invert,
    K::Saturate,
    K::Sepia,
    K::DropShadowX,
];

const DROP: [K; 3] = [K::DropShadowX, K::DropShadowY, K::DropShadowBlur];
const TEXT: [K; 3] = [K::TextShadowX, K::TextShadowY, K::TextShadowBlur];

pub(crate) fn aligned(a: u16, b: u16) -> bool {
    let (short, long) = if a.count_ones() <= b.count_ones() {
        (a, b)
    } else {
        (b, a)
    };
    let mut rest = long;
    let mut prefix = 0;
    for _ in 0..short.count_ones() {
        let lowest = rest & rest.wrapping_neg();
        prefix |= lowest;
        rest &= !lowest;
    }
    prefix == short
}

fn initial(kind: K, value: f32) -> bool {
    (value - kind.initial()).abs() < 1e-6
}

impl FilterStyle {
    pub(crate) fn value(&self, kind: K) -> Option<f32> {
        match kind {
            K::Blur => self
                .blur
                .filter(|blur| *blur > 0)
                .map(|blur| f32::from(blur) / super::SUB),
            K::HueRotate => self.hue.map(f32::from),
            K::DropShadowX | K::DropShadowY | K::DropShadowBlur => {
                self.drop.map(|spec| spec.part(kind))
            }
            K::TextShadowX | K::TextShadowY | K::TextShadowBlur => {
                self.single_text().map(|spec| spec.part(kind))
            }
            _ => AMOUNTS
                .iter()
                .find(|(_, owner)| *owner == kind)
                .and_then(|(amount, _)| self.amounts.get(*amount as usize).copied().flatten())
                .map(|percent| f32::from(percent) / 100.0),
        }
    }

    pub(crate) fn color(&self, kind: ColorAtomKind) -> Option<Rgba8> {
        match kind {
            ColorAtomKind::DropShadow => {
                self.drop.map(|spec| self.drop_color.unwrap_or(spec.color))
            }
            ColorAtomKind::TextShadow => self
                .single_text()
                .map(|spec| self.text_color.unwrap_or(spec.color)),
            _ => None,
        }
    }

    pub(crate) fn functions(&self) -> u16 {
        ORDER
            .iter()
            .enumerate()
            .filter(|(_, kind)| self.value(**kind).is_some())
            .fold(0, |list, (index, _)| list | 1 << index)
    }

    pub(crate) const fn text_count(&self) -> usize {
        match self.text {
            TextShadows::None => 0,
            TextShadows::Preset(list) => list.len(),
            TextShadows::One(_) => 1,
        }
    }

    const fn single_text(&self) -> Option<ShadowSpec> {
        match self.text {
            TextShadows::One(spec) => Some(spec),
            TextShadows::Preset([spec]) => Some(*spec),
            TextShadows::None | TextShadows::Preset(_) => None,
        }
    }

    pub(crate) fn overlay(&mut self, overlay: &dyn AnimatorOverlay) {
        let shown = |kind: K, present: bool| {
            overlay
                .value_for(kind)
                .filter(|value| present || !initial(kind, *value))
        };
        if let Some(px) = shown(K::Blur, self.value(K::Blur).is_some()) {
            self.blur = Some(blur_units(px));
        }
        for (slot, (_, kind)) in self.amounts.iter_mut().zip(AMOUNTS) {
            if let Some(value) = shown(kind, slot.is_some()) {
                *slot = Some(Amount::units(value));
            }
        }
        if let Some(degrees) = shown(K::HueRotate, self.hue.is_some()) {
            self.hue = Some(hue_units(degrees));
        }
        self.overlay_shadows(overlay);
    }

    fn overlay_shadows(&mut self, overlay: &dyn AnimatorOverlay) {
        let drop = self.drop.map(|spec| ShadowSpec {
            color: self.drop_color.unwrap_or(spec.color),
            ..spec
        });
        if let Some(spec) = moved(drop, (DROP, ColorAtomKind::DropShadow), overlay) {
            self.drop = Some(spec);
            self.drop_color = None;
        }
        if self.text_count() > 1 {
            return;
        }
        let text = self.single_text().map(|spec| ShadowSpec {
            color: self.text_color.unwrap_or(spec.color),
            ..spec
        });
        if let Some(spec) = moved(text, (TEXT, ColorAtomKind::TextShadow), overlay) {
            self.text = TextShadows::One(spec);
            self.text_color = None;
        }
    }
}

fn moved(
    base: Option<ShadowSpec>,
    (kinds, color_kind): ([K; 3], ColorAtomKind),
    overlay: &dyn AnimatorOverlay,
) -> Option<ShadowSpec> {
    let parts = kinds.map(|kind| overlay.value_for(kind));
    let color = overlay.value_for_color(color_kind);
    let changed = parts.iter().flatten().any(|value| value.abs() >= 1e-6)
        || color.is_some_and(|color| color.a > 0);
    if base.is_none() && !changed {
        return None;
    }
    if parts.iter().all(Option::is_none) && color.is_none() {
        return None;
    }
    let mut spec = base.unwrap_or(ShadowSpec::CLEAR);
    if let Some(x) = parts[0] {
        spec.offset[0] = length(x);
    }
    if let Some(y) = parts[1] {
        spec.offset[1] = length(y);
    }
    if let Some(blur) = parts[2] {
        spec.blur = blur_units(blur);
    }
    if let Some(color) = color {
        spec.color = color;
    }
    Some(spec)
}
