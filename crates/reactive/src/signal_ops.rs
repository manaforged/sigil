use core::fmt;
use core::ops::{
    AddAssign, BitAndAssign, BitOrAssign, BitXorAssign, DivAssign, MulAssign, RemAssign, ShlAssign,
    ShrAssign, SubAssign,
};

use crate::signal::Signal;

macro_rules! impl_assign_op {
    ($trait:ident, $method:ident) => {
        impl<T, U> $trait<U> for Signal<T>
        where
            T: $trait<U> + 'static,
        {
            #[inline]
            #[track_caller]
            fn $method(&mut self, rhs: U) {
                (*self).update(|inner| inner.$method(rhs));
            }
        }
    };
}

impl_assign_op!(AddAssign, add_assign);
impl_assign_op!(SubAssign, sub_assign);
impl_assign_op!(MulAssign, mul_assign);
impl_assign_op!(DivAssign, div_assign);
impl_assign_op!(RemAssign, rem_assign);
impl_assign_op!(BitAndAssign, bitand_assign);
impl_assign_op!(BitOrAssign, bitor_assign);
impl_assign_op!(BitXorAssign, bitxor_assign);
impl_assign_op!(ShlAssign, shl_assign);
impl_assign_op!(ShrAssign, shr_assign);

impl<T> fmt::Display for Signal<T>
where
    T: Clone + fmt::Display + 'static,
{
    #[inline]
    #[track_caller]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (*self).get().fmt(f)
    }
}
