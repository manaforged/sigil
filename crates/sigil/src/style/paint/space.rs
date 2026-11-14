use crate::style::StyleAtom;
use crate::style::atom::SpaceAtom;

use super::slot::push;
use crate::style::Paint;

pub(super) fn space<T: Paint>(target: T, atom: SpaceAtom) -> T {
    push(target, StyleAtom::Space(atom))
}

pub(super) fn finite_space<T: Paint>(target: T, make: fn(f32) -> SpaceAtom, value: f32) -> T {
    assert!(value.is_finite(), "2D transform values must be finite");
    space(target, make(value))
}

macro_rules! space_setters {
    () => {
        /// Moves the node horizontally by `px` pixels.
        #[must_use]
        fn translate_x(self, px: f32) -> Self {
            space::finite_space(self, SpaceAtom::TranslateX, px)
        }

        /// Moves the node vertically by `px` pixels.
        #[must_use]
        fn translate_y(self, px: f32) -> Self {
            space::finite_space(self, SpaceAtom::TranslateY, px)
        }

        /// Moves the node by `(x, y)` pixels.
        #[must_use]
        fn translate(self, x: f32, y: f32) -> Self {
            self.translate_x(x).translate_y(y)
        }

        /// Scales the node uniformly on both axes by `factor`.
        #[must_use]
        fn scale(self, factor: f32) -> Self {
            self.scale_x(factor).scale_y(factor)
        }

        /// Scales the node horizontally by `factor`.
        #[must_use]
        fn scale_x(self, factor: f32) -> Self {
            space::finite_space(self, SpaceAtom::ScaleX, factor)
        }

        /// Scales the node vertically by `factor`.
        #[must_use]
        fn scale_y(self, factor: f32) -> Self {
            space::finite_space(self, SpaceAtom::ScaleY, factor)
        }

        /// Turns the node in 2D by `degrees` (in degrees).
        #[must_use]
        fn rotate(self, degrees: f32) -> Self {
            space::finite_space(self, |deg| SpaceAtom::Rotate(deg.to_radians()), degrees)
        }

        /// Gives the children a 3D viewing distance in px, like Tailwind `perspective-*`.
        #[must_use]
        fn perspective(self, px: f32) -> Self {
            space::space(self, SpaceAtom::Perspective(Some(px.max(1.0))))
        }

        /// Sets the point the children are viewed from, as fractions of the box, like Tailwind `perspective-origin-*`.
        #[must_use]
        fn perspective_origin(self, x: f32, y: f32) -> Self {
            space::space(
                self,
                SpaceAtom::PerspectiveOrigin([x, y].map(fraction_units)),
            )
        }

        /// Turns the node around the x axis by `degrees` (in degrees), like Tailwind `rotate-x-*`.
        #[must_use]
        fn rotate_x(self, degrees: f32) -> Self {
            space::space(self, SpaceAtom::RotateX(degrees.to_radians()))
        }

        /// Turns the node around the y axis by `degrees` (in degrees), like Tailwind `rotate-y-*`.
        #[must_use]
        fn rotate_y(self, degrees: f32) -> Self {
            space::space(self, SpaceAtom::RotateY(degrees.to_radians()))
        }

        /// Turns the node around the z axis by `degrees` (in degrees), like Tailwind `rotate-z-*`.
        #[must_use]
        fn rotate_z(self, degrees: f32) -> Self {
            space::space(self, SpaceAtom::RotateZ(degrees.to_radians()))
        }

        /// Moves the node toward the viewer by `px`, like Tailwind `translate-z-*`.
        #[must_use]
        fn translate_z(self, px: f32) -> Self {
            space::space(self, SpaceAtom::TranslateZ(px))
        }

        /// Draws nothing while the node's back faces the viewer, like Tailwind `backface-hidden`.
        #[must_use]
        fn backface_hidden(self) -> Self {
            space::space(self, SpaceAtom::Backface(Backface::Hidden))
        }
        /// Shows the node's back when it turns away, like Tailwind `backface-visible`.
        #[must_use]
        fn backface_visible(self) -> Self {
            space::space(self, SpaceAtom::Backface(Backface::Visible))
        }
    };
}

pub(super) use space_setters;
