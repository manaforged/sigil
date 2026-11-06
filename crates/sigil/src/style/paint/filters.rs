use crate::scene::Shadow;
use crate::style::StyleAtom;
use crate::style::atom::{
    Amount, FilterAtom, FilterPart, ShadowSpec, TextShadows, blur_units, hue_units,
};

use super::slot::{PaintTarget, push};

pub(super) fn filter<T: PaintTarget>(target: T, part: FilterPart) -> T {
    push(target, StyleAtom::Filter(FilterAtom(part)))
}

pub(super) fn blur(px: f32) -> FilterPart {
    FilterPart::Blur(blur_units(px))
}

pub(super) fn amount(kind: Amount, value: f32) -> FilterPart {
    FilterPart::Amount(kind, Amount::units(value))
}

pub(super) fn hue_rotate(degrees: f32) -> FilterPart {
    FilterPart::HueRotate(hue_units(degrees))
}

pub(super) fn drop_shadow(shadow: Shadow) -> FilterPart {
    FilterPart::DropShadow(Some(ShadowSpec::of(shadow)))
}

pub(super) fn text_shadow(shadow: Option<Shadow>) -> FilterPart {
    FilterPart::TextShadow(shadow.map_or(TextShadows::None, |shadow| {
        TextShadows::One(ShadowSpec::of(shadow))
    }))
}

macro_rules! filter_setters {
    () => {
        /// Blurs the node and its children by a standard deviation of `px`, like Tailwind `blur-*`.
        #[must_use]
        fn blur(self, px: f32) -> Self {
            filters::filter(self, filters::blur(px))
        }

        /// Scales the brightness, 1.0 unchanged, like Tailwind `brightness-*`.
        #[must_use]
        fn brightness(self, amount: f32) -> Self {
            filters::filter(self, filters::amount(Amount::Brightness, amount))
        }

        /// Scales the contrast, 1.0 unchanged, like Tailwind `contrast-*`.
        #[must_use]
        fn contrast(self, amount: f32) -> Self {
            filters::filter(self, filters::amount(Amount::Contrast, amount))
        }

        /// Mixes toward gray, 1.0 fully gray, like Tailwind `grayscale-*`.
        #[must_use]
        fn grayscale(self, amount: f32) -> Self {
            filters::filter(self, filters::amount(Amount::Grayscale, amount))
        }

        /// Rotates the hue by `degrees`, like Tailwind `hue-rotate-*`.
        #[must_use]
        fn hue_rotate(self, degrees: f32) -> Self {
            filters::filter(self, filters::hue_rotate(degrees))
        }

        /// Mixes toward the inverted colors, 1.0 fully inverted, like Tailwind `invert-*`.
        #[must_use]
        fn invert(self, amount: f32) -> Self {
            filters::filter(self, filters::amount(Amount::Invert, amount))
        }

        /// Scales the saturation, 1.0 unchanged, like Tailwind `saturate-*`.
        #[must_use]
        fn saturate(self, amount: f32) -> Self {
            filters::filter(self, filters::amount(Amount::Saturate, amount))
        }

        /// Mixes toward sepia, 1.0 fully sepia, like Tailwind `sepia-*`.
        #[must_use]
        fn sepia(self, amount: f32) -> Self {
            filters::filter(self, filters::amount(Amount::Sepia, amount))
        }

        /// Draws `shadow` under the node's painted pixels, its blur a standard deviation, like Tailwind `drop-shadow-*`.
        #[must_use]
        fn drop_shadow(self, shadow: Shadow) -> Self {
            filters::filter(self, filters::drop_shadow(shadow))
        }

        /// Draws `shadow` under the node's glyphs, its blur a standard deviation, like Tailwind `text-shadow-*`; `None` cancels an inherited shadow.
        #[must_use]
        fn text_shadow(self, shadow: impl Into<Option<Shadow>>) -> Self {
            filters::filter(self, filters::text_shadow(shadow.into()))
        }
    };
}

pub(super) use filter_setters;
