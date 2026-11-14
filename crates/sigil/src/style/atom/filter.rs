#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "shadow lengths are clamped to the i16 and u16 ranges before the cast"
)]

use smallvec::SmallVec;

use super::Color;
use crate::scene::Shadow;
use crate::scene::command::Filter;
use crate::style::NumericAtomKind as K;
use crate::style::srgb::{decode_rgb, encode, unit};

mod motion;

pub(crate) use motion::aligned;

const SUB: f32 = 16.0;

fn length(value: f32) -> i16 {
    (value * SUB).round().clamp(-32_768.0, 32_767.0) as i16
}

pub(crate) fn blur_units(px: f32) -> u16 {
    (px.max(0.0) * SUB).round().min(65_535.0) as u16
}

pub(crate) fn hue_units(degrees: f32) -> i16 {
    (degrees % 360.0).round() as i16
}

pub(crate) fn shadow_color(shadow: Shadow) -> Color {
    let [r, g, b, a] = shadow.color;
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color {
        r: byte(encode(r)),
        g: byte(encode(g)),
        b: byte(encode(b)),
        a: byte(a),
    }
}

kittest_pub! {
    #[doc(hidden)]
    /// A filter or text shadow style atom.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FilterAtom(pub(crate) FilterPart);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ShadowSpec {
    offset: [i16; 2],
    blur: u16,
    color: Color,
}

impl ShadowSpec {
    pub(crate) const fn black(y: i16, sigma_16ths: u16, alpha: u8) -> Self {
        Self {
            offset: [0, y * 16],
            blur: sigma_16ths,
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: alpha,
            },
        }
    }

    pub(crate) fn of(shadow: Shadow) -> Self {
        Self {
            offset: shadow.offset.map(length),
            blur: blur_units(shadow.blur),
            color: shadow_color(shadow),
        }
    }

    const CLEAR: Self = Self {
        offset: [0, 0],
        blur: 0,
        color: Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
    };

    fn part(self, kind: K) -> f32 {
        match kind {
            K::DropShadowX | K::TextShadowX => f32::from(self.offset[0]) / SUB,
            K::DropShadowY | K::TextShadowY => f32::from(self.offset[1]) / SUB,
            _ => f32::from(self.blur) / SUB,
        }
    }

    fn shadow(self, color: Option<Color>) -> Shadow {
        let color = color.unwrap_or(self.color);
        let [r, g, b] = decode_rgb(color);
        let linear = [r, g, b, unit(color.a)];
        let offset = self.offset.map(|value| f32::from(value) / SUB);
        Shadow::new(offset, f32::from(self.blur) / SUB, 0.0, linear)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TextShadows {
    Unset,
    None,
    Preset(&'static [ShadowSpec]),
    One(ShadowSpec),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FilterPart {
    Blur(u16),
    Amount(Amount, u16),
    HueRotate(i16),
    DropShadow(Option<ShadowSpec>),
    TextShadow(TextShadows),
    Backdrop(BackdropPart, u16),
    Shader(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BackdropPart {
    Blur,
    Brightness,
    Saturate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Amount {
    Brightness,
    Contrast,
    Grayscale,
    Invert,
    Saturate,
    Sepia,
}

type AmountFilter = (Amount, fn(f32) -> Filter);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FilterStyle {
    blur: Option<u16>,
    amounts: [Option<u16>; 6],
    hue: Option<i16>,
    drop: Option<ShadowSpec>,
    text: TextShadows,
    backdrop: [Option<u16>; 3],
    pub(crate) shader: Option<u64>,
    pub(crate) drop_color: Option<Color>,
    pub(crate) text_color: Option<Color>,
}

const BACKDROP: [(K, fn(f32) -> Filter); 3] = [
    (K::BackdropBlur, Filter::Blur),
    (K::BackdropBrightness, Filter::Brightness),
    (K::BackdropSaturate, Filter::Saturate),
];

fn backdrop_index(kind: K) -> Option<usize> {
    BACKDROP.iter().position(|(owner, _)| *owner == kind)
}

impl Amount {
    pub(crate) fn units(value: f32) -> u16 {
        (value.max(0.0) * 100.0).round().min(65_535.0) as u16
    }
}

impl FilterStyle {
    pub(crate) const NONE: Self = Self {
        blur: None,
        amounts: [None; 6],
        hue: None,
        drop: None,
        text: TextShadows::Unset,
        backdrop: [None; 3],
        shader: None,
        drop_color: None,
        text_color: None,
    };

    pub(crate) fn apply(&mut self, part: FilterPart) {
        match part {
            FilterPart::Blur(sixteenths) => self.blur = Some(sixteenths),
            FilterPart::Amount(amount, percent) => {
                if let Some(slot) = self.amounts.get_mut(amount as usize) {
                    *slot = Some(percent);
                }
            }
            FilterPart::HueRotate(degrees) => self.hue = Some(degrees),
            FilterPart::DropShadow(shadow) => self.drop = shadow,
            FilterPart::TextShadow(shadows) => self.text = shadows,
            FilterPart::Shader(id) => self.shader = Some(id),
            FilterPart::Backdrop(part, units) => {
                if let Some(slot) = self.backdrop.get_mut(part as usize) {
                    *slot = Some(units);
                }
            }
        }
    }

    pub(crate) fn backdrop(&self) -> SmallVec<[Filter; 4]> {
        BACKDROP
            .iter()
            .filter_map(|(kind, make)| self.backdrop_value(*kind).map(make))
            .collect()
    }

    pub(crate) fn backdrop_value(&self, kind: K) -> Option<f32> {
        let units = self
            .backdrop
            .get(backdrop_index(kind)?)
            .copied()
            .flatten()?;
        match kind {
            K::BackdropBlur => (units > 0).then(|| f32::from(units) / SUB),
            _ => Some(f32::from(units) / 100.0),
        }
    }

    pub(crate) fn overlay_backdrop(&mut self, shown: &dyn Fn(K, bool) -> Option<f32>) {
        for (kind, _) in BACKDROP {
            let present = self.backdrop_value(kind).is_some();
            let Some(value) = shown(kind, present) else {
                continue;
            };
            let units = match kind {
                K::BackdropBlur => blur_units(value),
                _ => Amount::units(value),
            };
            if let Some(slot) = backdrop_index(kind).and_then(|at| self.backdrop.get_mut(at)) {
                *slot = Some(units);
            }
        }
    }

    pub(crate) fn filters(&self) -> SmallVec<[Filter; 4]> {
        let mut list = SmallVec::new();
        if let Some(blur) = self.blur.filter(|blur| *blur > 0) {
            list.push(Filter::Blur(f32::from(blur) / SUB));
        }
        let amount = |index: Amount| {
            self.amounts
                .get(index as usize)
                .copied()
                .flatten()
                .map(|n| f32::from(n) / 100.0)
        };
        let colors: [AmountFilter; 3] = [
            (Amount::Brightness, Filter::Brightness),
            (Amount::Contrast, Filter::Contrast),
            (Amount::Grayscale, Filter::Grayscale),
        ];
        list.extend(
            colors
                .iter()
                .filter_map(|(kind, make)| amount(*kind).map(make)),
        );
        list.extend(
            self.hue
                .map(|degrees| Filter::HueRotate(f32::from(degrees))),
        );
        let tones: [AmountFilter; 3] = [
            (Amount::Invert, Filter::Invert),
            (Amount::Saturate, Filter::Saturate),
            (Amount::Sepia, Filter::Sepia),
        ];
        list.extend(
            tones
                .iter()
                .filter_map(|(kind, make)| amount(*kind).map(make)),
        );
        list.extend(
            self.drop
                .map(|spec| Filter::DropShadow(spec.shadow(self.drop_color))),
        );
        list
    }

    pub(crate) const fn inherits_text_shadows(&self) -> bool {
        matches!(self.text, TextShadows::Unset)
    }

    pub(crate) fn text_shadows(&self) -> SmallVec<[Shadow; 3]> {
        let specs: &[ShadowSpec] = match &self.text {
            TextShadows::Unset | TextShadows::None => &[],
            TextShadows::Preset(list) => list,
            TextShadows::One(spec) => std::slice::from_ref(spec),
        };
        specs
            .iter()
            .map(|spec| spec.shadow(self.text_color))
            .collect()
    }
}

impl Default for FilterStyle {
    fn default() -> Self {
        Self::NONE
    }
}
