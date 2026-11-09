#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "shadow lengths are clamped to the i16 and u16 ranges before the cast"
)]

use smallvec::SmallVec;

use super::Rgba8;
use crate::scene::Shadow;
use crate::scene::command::Filter;
use crate::style::NumericAtomKind;
use crate::style::srgb::{decode, encode};

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

pub(crate) fn shadow_color(shadow: Shadow) -> Rgba8 {
    let [r, g, b, a] = shadow.color;
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgba8 {
        r: byte(encode(r)),
        g: byte(encode(g)),
        b: byte(encode(b)),
        a: byte(a),
    }
}

/// A filter or text shadow style atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FilterAtom(pub(crate) FilterPart);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ShadowSpec {
    offset: [i16; 2],
    blur: u16,
    color: Rgba8,
}

impl ShadowSpec {
    pub(crate) const fn black(y: i16, sigma_16ths: u16, alpha: u8) -> Self {
        Self {
            offset: [0, y * 16],
            blur: sigma_16ths,
            color: Rgba8 {
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
        color: Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
    };

    fn part(self, kind: NumericAtomKind) -> f32 {
        match kind {
            NumericAtomKind::DropShadowX | NumericAtomKind::TextShadowX => {
                f32::from(self.offset[0]) / SUB
            }
            NumericAtomKind::DropShadowY | NumericAtomKind::TextShadowY => {
                f32::from(self.offset[1]) / SUB
            }
            _ => f32::from(self.blur) / SUB,
        }
    }

    fn shadow(self, color: Option<Rgba8>) -> Shadow {
        let color = color.unwrap_or(self.color);
        let linear = [
            decode(color.r),
            decode(color.g),
            decode(color.b),
            f32::from(color.a) / 255.0,
        ];
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
    pub(crate) drop_color: Option<Rgba8>,
    pub(crate) text_color: Option<Rgba8>,
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
