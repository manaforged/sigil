use super::{
    Fill, Flex, Geometry, Sizing, Spacing, StateVariants, StyleAccess, Transitions, Typography,
    Visual,
};

pub trait Styled:
    StyleAccess
    + Spacing
    + Sizing
    + Flex
    + Fill
    + Typography
    + Geometry
    + Visual
    + Transitions
    + StateVariants
{
}

impl<T> Styled for T where
    T: StyleAccess
        + Spacing
        + Sizing
        + Flex
        + Fill
        + Typography
        + Geometry
        + Visual
        + Transitions
        + StateVariants
{
}
