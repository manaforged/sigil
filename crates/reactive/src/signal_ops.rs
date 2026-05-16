//! Ergonomic operator overloads and formatting for [`Signal<T>`].
//!
//! Lets reactive code at the call site read like ordinary value
//! mutation and formatting:
//!
//! ```text
//! let count = signal(0_i32);
//! button("inc").on_click(move |_| count += 1);   // AddAssign
//! label(move || format!("Count: {count}"));     // Display
//! ```
//!
//! Each compound-assignment operator (`+=`, `-=`, `*=`, ...) forwards
//! to [`Signal::update`], which preserves the panic-on-recursion guard
//! and effect-fan-out semantics. [`Display`] forwards to
//! [`Signal::get`] so a format inside a reactive scope subscribes the
//! enclosing effect; outside one it simply reads.
//!
//! All impls require `T: 'static` (the underlying signal storage
//! invariant) plus the matching operator trait on `T` (for the
//! assign-ops) or `Clone + Display` (for [`Display`]).
//!
//! Per DR-005 these are pure ergonomic helpers — they add no new
//! reactive contract, only call existing primitives.

use core::fmt;
use core::ops::{
    AddAssign, BitAndAssign, BitOrAssign, BitXorAssign, DivAssign, MulAssign, RemAssign, ShlAssign,
    ShrAssign, SubAssign,
};

use crate::signal::Signal;

/// Generate an `impl <Trait>Assign<U> for Signal<T>` that forwards to
/// `Signal::update`, mutating the underlying value in place.
macro_rules! impl_assign_op {
    ($trait:ident, $method:ident) => {
        impl<T, U> $trait<U> for Signal<T>
        where
            T: $trait<U> + 'static,
        {
            #[inline]
            #[track_caller]
            fn $method(&mut self, rhs: U) {
                // `Signal<T>: Copy` — deref to call `update` by value
                // without disturbing the `&mut self` binding the
                // operator desugar gives us.
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

/// `Display` for `Signal<T>` reads the inner value via [`Signal::get`]
/// — so `format!("{count}")` inside an effect subscribes that effect
/// to the signal, exactly like an explicit `count.get()` would.
///
/// Outside a reactive scope, [`Signal::get`] reads without
/// subscribing.
///
/// # Panics
///
/// Panics if the signal has been disposed, mirroring [`Signal::get`].
///
/// # Borrow caveat
///
/// `Display::fmt` calls [`Signal::get`], which briefly borrows the
/// runtime arena. Formatting a signal from inside a callback that
/// *already* holds a runtime borrow (e.g. a custom `Drop` impl run
/// during a `Signal::update` closure that itself triggers a format)
/// will panic with a `BorrowMutError`. The signal-level recursion
/// guard catches the same-signal case; this is the rarer
/// cross-signal case. Normal use — formatting inside an effect body,
/// an event handler, or a render closure — does not trip the
/// borrow because the runtime borrow is released before user code
/// runs.
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
